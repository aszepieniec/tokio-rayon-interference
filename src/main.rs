use rand::Rng;
use rand::RngCore;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::ThreadPoolBuilder;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::iter;
use std::time::Duration;

/// Number of leading zeros for a valid guess.
const DIFFICULTY: usize = 8;

fn main() {
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Could not create tokio runtime");

    tokio_runtime.block_on(async {
        // activate exactly one
        // let guess_function = seq_guess;
        // let guess_function = par_guess_global;
        let guess_function = par_guess_segregated;

        let guesser_task = tokio::task::spawn_blocking(guess_function);
        std::thread::sleep(Duration::from_secs_f64(1.5));
        let verifier_task = tokio::task::spawn_blocking(verify);
        let verdict = verifier_task.await.unwrap();
        println!("got verdict: {verdict}");
        let winning_guess = guesser_task.await.unwrap();
        println!("winning guess found: {winning_guess}");
    });

    tokio_runtime.shutdown_timeout(tokio::time::Duration::from_secs(10));
}

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
        .find_any(|r| *r < (u64::MAX >> DIFFICULTY))
        .unwrap()
}

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
            .find_any(|r| *r < (u64::MAX >> DIFFICULTY))
            .unwrap()
    })
}

fn seq_guess() -> u64 {
    iter::repeat(0)
        .map(|_| {
            std::thread::sleep(Duration::from_secs(1));
            let randomness = rand::thread_rng().next_u64();
            println!("guessing single-threaded -- randomness: {}", randomness);
            randomness
        })
        .find(|r| *r < (u64::MAX >> DIFFICULTY))
        .unwrap()
}

fn verify() -> bool {
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
