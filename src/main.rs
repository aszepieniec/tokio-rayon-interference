use rand::Rng;
use rand::RngCore;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::ThreadPoolBuilder;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::iter;
use std::time::Duration;
use tokio::task::spawn_blocking;

/// Number of leading zeros for a valid guess.
const DIFFICULTY: usize = 8;
const TARGET: u64 = u64::MAX >> DIFFICULTY;

fn main() {
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Could not create tokio runtime");

    tokio_runtime.block_on(async {
        // spawn guesser task
        // ** activate exactly one of the next three lines: **
        let guess_function = par_guess_global;
        // let guess_function = seq_guess;
        // let guess_function = par_guess_segregated;
        let guesser_task = tokio::task::spawn_blocking(move || {
            let winning_guess = guess_function();
            println!("**** got winning guess: {winning_guess} ****");
            winning_guess
        });

        // wait 1.5 s
        std::thread::sleep(Duration::from_secs_f64(1.5));

        // spawn verify task
        // ** activate exactly one of the next four lines: **
        let verdict = par_verify_spawn().await;
        // let verdict = spawn_blocking(par_verify_direct).await.unwrap();
        // let verdict = seq_verify_spawn().await;
        // let verdict = spawn_blocking(seq_verify_direct).await.unwrap();

        // block until verify is done
        println!("got verdict: {verdict}\n");

        // block until guessing is done
        let winning_guess = guesser_task.await.unwrap();
        println!("winning guess found: {winning_guess}");
    });

    tokio_runtime.shutdown_timeout(tokio::time::Duration::from_secs(10));
}

/// Guess in parallel, using rayon's global thead-pool.
fn par_guess_global() -> u64 {
    rayon::iter::repeat(0)
        .map_init(rand::thread_rng, |rng, _| {
            std::thread::sleep(Duration::from_secs(1));
            let randomness = rng.next_u64();
            println!(
                "guessing (global) thread {} -- randomness: {}",
                rayon::current_thread_index().unwrap(),
                randomness
            );
            randomness
        })
        .find_any(|r| *r < TARGET)
        .unwrap()
}

/// Guess in parallel, using a segregated rayon thread-pool.
fn par_guess_segregated() -> u64 {
    let pool = ThreadPoolBuilder::new()
        .num_threads(rayon::current_num_threads())
        .build()
        .unwrap();
    pool.install(|| {
        rayon::iter::repeat(0)
            .map_init(rand::thread_rng, |rng, _| {
                std::thread::sleep(Duration::from_secs(1));
                let randomness = rng.next_u64();
                println!(
                    "guessing (segregated) thread {} -- randomness: {}",
                    rayon::current_thread_index().unwrap(),
                    randomness
                );
                randomness
            })
            .find_any(|r| *r < TARGET)
            .unwrap()
    })
}

/// Guess sequentially.
fn seq_guess() -> u64 {
    let num_threads = rayon::current_num_threads();
    iter::repeat(0)
        .map(|_| {
            std::thread::sleep(Duration::from_secs_f64(1.0 / (num_threads as f64)));
            let randomness = rand::thread_rng().next_u64();
            println!("guessing single-threaded -- randomness: {}", randomness);
            randomness
        })
        .find(|r| *r < TARGET)
        .unwrap()
}

/// Verify in parallel in the current task, using rayon's global thread-pool.
fn par_verify_direct() -> bool {
    let seed = rand::thread_rng().r#gen::<[u8; 32]>();
    let verdict = (0..=u8::MAX)
        .into_par_iter()
        .map(|c| {
            let mut local_seed = seed;
            *local_seed.last_mut().unwrap() = local_seed.last().unwrap().wrapping_add(c);
            let mut local_rng = StdRng::from_seed(local_seed);
            local_rng.next_u64()
        })
        .all(|i| i > 0);
    println!("**** done with verification; verdict: {} ****", verdict);
    verdict
}

/// Verify in parallel in a spawned task, using rayon's global thread-pool.
async fn par_verify_spawn() -> bool {
    tokio::task::spawn_blocking(par_verify_direct)
        .await
        .expect("should be able to spawn parallel verify task")
}

/// Verify sequentially in the current task.
fn seq_verify_direct() -> bool {
    let seed = rand::thread_rng().r#gen::<[u8; 32]>();
    let verdict = (0..=u8::MAX)
        .map(|c| {
            let mut local_seed = seed;
            *local_seed.last_mut().unwrap() = local_seed.last().unwrap().wrapping_add(c);
            let mut local_rng = StdRng::from_seed(local_seed);
            local_rng.next_u64()
        })
        .all(|i| i > 0);
    println!(
        "**** done with sequential verification; verdict: {} ****",
        verdict
    );
    verdict
}

/// Verify sequantially in a spawned task.
async fn seq_verify_spawn() -> bool {
    tokio::task::spawn_blocking(seq_verify_direct)
        .await
        .expect("should be able to spawn sequential verify task")
}
