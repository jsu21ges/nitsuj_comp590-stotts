use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use std::io::{self, Write};
use rand::Rng;

const NUM_PHILOSOPHERS: usize = 5;

// A struct to track the state of a philosopher
struct Philosopher {
    eat_count: usize,
    think_count: usize,
}

impl Philosopher {
    fn new() -> Self {
        Philosopher {
            eat_count: 0,
            think_count: 0,
        }
    }

    fn eat(&mut self) {
        self.eat_count += 1;
    }

    fn think(&mut self) {
        self.think_count += 1;
    }
}

// Acts as a standin for a mutex lock
struct Fork;

impl Fork {
    fn new() -> Self {
        Fork
    }
}

fn random_in_range(min: u64, max: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

// Philosopher thread function
fn philosopher_thread(
    philosopher: Arc<Mutex<Philosopher>>,
    forks: Vec<Arc<Mutex<Fork>>>,
    stop_signal: Arc<Mutex<bool>>, // Shared stop signal to stop philosophers
    philosopher_id: usize,
) {
    loop {
        // Check if stop signal is active
        if *stop_signal.lock().unwrap() {
            println!("Philosopher {} is stopping.", philosopher_id);
            break;
        }

        // Thinking phase
        {
            let mut philosopher = philosopher.lock().unwrap();
            philosopher.think();
        }
        let thinking_time = random_in_range(1, 3);
        println!("Philosopher {} is thinking for {} seconds.", philosopher_id, thinking_time);
        thread::sleep(Duration::from_secs(thinking_time));

        // Check if the stop signal was received before eating
        if *stop_signal.lock().unwrap() {
            println!("Philosopher {} is stopping after thinking.", philosopher_id);
            break;
        }

        // Pick up forks in a fixed order (even num philosophers do left then right, odd number philosophers do right then left.)
        let (left_fork_id, right_fork_id) = if philosopher_id % 2 == 0 {
            (philosopher_id, (philosopher_id + 1) % NUM_PHILOSOPHERS) 
        } else {
            ((philosopher_id + 1) % NUM_PHILOSOPHERS, philosopher_id)
        };

        let left_fork = &forks[left_fork_id];
        let right_fork = &forks[right_fork_id];

        // Try to lock the forks
        let _left = left_fork.lock().unwrap(); // Lock left fork
        let _right = right_fork.lock().unwrap(); // Lock right fork

        {
            let mut philosopher = philosopher.lock().unwrap();
            philosopher.eat();
        }

        let eating_time = random_in_range(1, 5);
        println!("Philosopher {} is eating for {} seconds.", philosopher_id, eating_time);
        thread::sleep(Duration::from_secs(eating_time));

        // Check again for the stop signal after eating
        if *stop_signal.lock().unwrap() {
            println!("Philosopher {} is stopping after eating.", philosopher_id);
            break;
        }
    }
}

// Function to handle user input in a separate thread
fn input_thread(stop_sender: mpsc::Sender<()>) {
    loop {
        println!("Enter 'exit' to stop: ");
        io::stdout().flush().unwrap(); // Ensure prompt is displayed immediately
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if input.trim() == "exit" {
            // Send stop signal to the main thread immediately
            stop_sender.send(()).unwrap();
            break;
        }
    }
}

fn main() {
    // Create forks and philosophers
    let forks: Vec<Arc<Mutex<Fork>>> = (0..NUM_PHILOSOPHERS)
        .map(|_i| Arc::new(Mutex::new(Fork::new())))
        .collect();

    let philosophers: Vec<Arc<Mutex<Philosopher>>> = (0..NUM_PHILOSOPHERS)
        .map(|_i| Arc::new(Mutex::new(Philosopher::new())))
        .collect();

    // Shared stop signal
    let stop_signal = Arc::new(Mutex::new(false));

    // Create a channel for stopping the philosophers
    let (stop_sender, stop_receiver) = mpsc::channel::<()>();

    // Create philosopher threads
    let mut handles = vec![];
    for i in 0..NUM_PHILOSOPHERS {
        let philosopher = philosophers[i].clone();
        let forks = forks.clone();
        let stop_signal = stop_signal.clone();
        let handle = thread::spawn(move || {
            philosopher_thread(philosopher, forks, stop_signal, i);
        });
        handles.push(handle);
    }

    // Spawn the input thread for receiving user input
    let stop_sender_clone = stop_sender.clone();
    thread::spawn(move || {
        input_thread(stop_sender_clone);
    });

    // Wait for the stop signal from the input thread
    stop_receiver.recv().unwrap();  // Blocking call until the stop signal is received
    println!("Received stop signal. Stopping all philosophers.");

    // Set the stop signal to true, causing all philosophers to stop
    *stop_signal.lock().unwrap() = true;

    // Wait for all philosopher threads to finish
    for handle in handles {
        handle.join().unwrap();
    }

    // Print the final counts for each philosopher
    for i in 0..NUM_PHILOSOPHERS {
        let philosopher = philosophers[i].lock().unwrap();
        println!(
            "Philosopher {}: Ate {} times, Thought {} times",
            i,
            philosopher.eat_count,
            philosopher.think_count
        );
    }
}
