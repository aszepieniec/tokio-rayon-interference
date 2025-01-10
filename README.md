# Tokio/Rayon Interference

Minimal example exhibiting how competing demands on rayon's thread pool, coming from separate tokio tasks, can interfere with each other resulting one stalling task, and a potential resolution.

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

