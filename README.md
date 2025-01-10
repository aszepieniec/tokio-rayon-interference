# Tokio/Rayon Interference

Minimal example exhibiting how competing demands on rayon's thread pool, coming from separate tokio tasks, can interfere with each other resulting one stalling task, and potential resolutions.

## Setting

We have an asynchronous application whose concurrency is managed by [tokio](https://tokio.rs/). There are separate tasks that need to do work concurrently.

 1. *Guessing.* Originally stemming from proof-of-work mining. The task is to find for some hash function a (pre-image, after-image) pair satisfying a sparse relation such as `after_image < target`. The details that are relevant for this demo are
  - guessing is embarrassingly parallelizable;
  - guessing must run until success, or until the task is killed externally, either of which can take a while.

 2. *Verifying.* Originally stemming from verification of succinct arguments of knowledge. Verification involves elementary arithmetic over finite fields and hash function evaluations. These steps can be parallelized to some degree. The important factors for this demo:
  - the workload is finite -- in other words, there is a fixed number of finite field operations and hash function evaluations;
  - even a sequential run is relatively fast, on the order of 50 ms.

We implemented this application naïvely using [rayon](https://docs.rs/rayon/latest/rayon/)'s parallel iterators. We observed the following:

 > if we initiate a `verify` task while a `guessing` task is running, then the `verify` task stalls seemingly indefinitely.

After determining (what we think is) the root cause, the qualifier "seemingly" before "indefinitely" merely indicates that in practice the expected duration of guessing was impractically large. In concrete terms, the `target` was too small to have justify hope of the guesser task running in a short period of time.

## Root Cause

The root cause, we think, is the following. The guesser task and the verifier task both demand resources from a single global rayon thread-pool. The guesser task comes first and keeps spawning jobs until it finds a successful guess. The verifier task adds its jobs to the queue, but these are never executed because the new gesser jobs have priority.

To test this explanation, I wrote this minimal demo. I find that it explains the observations quite well.

## Demo

The file `main.rs` contains a minimal working example that reproduces this observation along with potential cures. A guesser task is started, and then 1.5 second later, a verify task is started. Depending on the configuration, the verify task completes either *after* the the guesser task is done, or *while it is running*. When the difficulty/target is well configured, the second case entails a completed verify task long before the guess task is finished. The second order of events is the desired behavior; the first was the motivation for the whole bug hunt that led to this demo.

Both tasks write text to stdout. Based on the timing of this text, one can infer the likely order of events. Note that there is some noise in between tasks terminating and stdout flushing, so it is imperative to configure the difficulty appropriately in order to magnify the signal.

### Configuration

The parameter `DIFFICULTY` regulates the ($\log_2$ of the) expected number of guessed before finding a valid pre-image, and, consequently, the expected duration of the guess task.

There are three configurations for the guess task:
 1. naïve parallelism using rayon's global thread-pool (<-- original error-triggering configuration);
 2. sequential;
 3. parallelism using a segregated rayon thread-pool (<-- proposed solution).

There are two configurations for spawning the verify task as far as tokio is concerned:
 1. directly from the main task using `.await`;
 2. in a separate task using `spawn_blocking`.

There are two modes for the verify task in terms of parallelism:
 1. parallelism using rayon's global thread-pool;
 2. sequential.

Note that using a segregated rayon thread-pool for the verify task is not possible configuration because (a) in our case the verifier function lives in a separate library; and (b) this configuration adds no explaining power to the issue at hand. Nevertheless, we do expect that configuration to yield a viable solution also.

To select the configuration, look for the lines marked with `**` asterisks `**` and comment/un-comment the right line below.

### Observations

The observations across all configurations can be condensed to two propositions:

 1. *Whenever the guess and the verify task are parallelized with the global rayon thread-pool, the verify task ends up stalling.* This proposition supports the hypothesized root cause: the jobs originating from the verify task are never executed because the guess tasks endlessly spawns new jobs that have priority.
 2. *Whether the verify task is invoked directly from the main task, or whether a separate task is spawned for it, is irrelevant to the observed behavior.* Note that it is recommended to use `spawn_blocking` for CPU-bound code that takes more than 10 ms or so to complete. And that's probably good advice in general and even in this case, but the point is that in this particular case it does nothing to resolve the issue. It merely contains the stall to a new task, as opposed to stalling the main task where it will surely be noticed.

## Conclusion

Based on the available evidence, the issue arises from competing and concurrent demands on rayon's global thread-pool. Its resources are served to the first client and not shared with latecomers until the first is done. Reliable fixes are a) to make one or both tasks sequential, or b) to use a segregated rayon thread-pool for at least one task.

I confess I'm uncertain as to *why* this behavior occurs. One would naïvely expect rayon's global thread-pool to be shared between early birds and late comers. I *suspect* (but did not confirm) that the *de facto* queue comes about as a result of rayon's *sequential* work management. By engaging a separate thread-pool we get distinct sequential work managers who take turns, courtesy of tokio.
