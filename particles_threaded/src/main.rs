use rand::random;
use threadpool::ThreadPool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const THREAD_COUNT : usize = 10;
const COLLISION_THREAD_COUNT : usize = 1;
const PARTICLE_COUNT : usize = 100;

const ENCLOSURE_W : f32 = 10.0;
const ENCLOSURE_H : f32 = 10.0;

const PARTICLE_RADIUS_SQUARED : f32 = 0.01; // (0.01 == 0.1^2 which saves square rooting the distance)

// Simulation values
const SIMULATION_TIME_SECONDS : f32 = 10.0;

#[derive(Debug, Copy, Clone)]
struct Particle {
    x: f32,
    y: f32,
}

impl Particle {

    // Compare the distance between two particles, if the distance is less than 0.1, they have collided
    fn perform_collision_check(&self, other_particle: &Particle) -> bool {
        let dist_x = self.x - other_particle.x;
        let dist_y = self.y - other_particle.y;
        let squared_distance = dist_x * dist_x + dist_y * dist_y;

        return squared_distance < PARTICLE_RADIUS_SQUARED;
    }
}

struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    fn new() -> Self {
        let mut created_particles = Vec::new();
        
        for _ in 0..PARTICLE_COUNT {
            created_particles.push(Particle {
                x: 0.0,
                y: 0.0
            });
        }

        ParticleSystem { particles: created_particles }
    }

    // Print all particles and their positions to the console 
    fn debug_print_particles(& self) {
        let mut i = 0;
        for p in & self.particles {
            if i % 5 == 0 && i != 0 {
                println!("{} : x {} y {}", i, p.x, p.y);
            } else {
                print!("{} : x {} y {} |", i, p.x, p.y);
            }
            i+=1;
        }

        println!("\n----");
    }
}

// Move all particles by a random amount inside the enclosure
fn random_move_particles(particle_list: &mut[Particle]){
    for p in particle_list {
        p.x = random::<f32>() * ENCLOSURE_W;
        p.y = random::<f32>() * ENCLOSURE_H;
    }
}

fn move_thread_main(particle_system: Arc<Mutex<ParticleSystem>>, start: usize, len: usize){
    let mut iterations: u32 = 0;
    let start_time = Instant::now();

    let mut local_chunk : Vec<Particle> = { // Use scoped set to let the lock go out of scope
        let system = particle_system.lock().unwrap();
        system.particles[start..start + len].to_vec()
    };

    while start_time.elapsed().as_secs_f32() < SIMULATION_TIME_SECONDS {
        random_move_particles(&mut local_chunk);

        let mut system = particle_system.lock().unwrap(); // Only lock to update the local chunk
        system.particles[start .. start + len].copy_from_slice(&local_chunk);

        iterations+=1;
    }

    println!("Ran {} in {}s", iterations, SIMULATION_TIME_SECONDS)
}

fn collision_thread_main(particle_system: Arc<Mutex<ParticleSystem>>) {
    let start_time = Instant::now();

    let mut collision_count : usize = 0;

    while start_time.elapsed().as_secs_f32() < SIMULATION_TIME_SECONDS {
        // Temporarily lock mutex to access particles and then release - use as "snapshot" of collisions occuring
        let particles : Vec<Particle> = { // Use scoped set to let the lock go out of scope
            let system = particle_system.lock().unwrap();
            system.particles.to_vec()
        };

        for i in 0..particles.len() {
            for j in i + 1..particles.len() {
                if particles[i].perform_collision_check(&particles[j]) {
                    collision_count += 1;
                }
            }
        }
    }

    println!("{} collisions occured", collision_count);
}

fn main() {
    let particle_system_mut = Arc::new(Mutex::new(ParticleSystem::new()));
    let particles_len = particle_system_mut.lock().unwrap().particles.len();

    let chunk_size = particles_len / THREAD_COUNT; // Split data into equal chunks

    let pool = ThreadPool::new(THREAD_COUNT); // Create thread pool
    let collision_pool = ThreadPool::new(COLLISION_THREAD_COUNT);

    // Instance the random move threads
    for i in 0..THREAD_COUNT {
        let system_clone = Arc::clone(&particle_system_mut);

        let chunk_idx = i * chunk_size;
        let mut chunk_len = chunk_size;

        if chunk_idx + chunk_len > particles_len - 1 {
            chunk_len = particles_len - chunk_idx - 1;
        }

        pool.execute(move || move_thread_main(system_clone, chunk_idx, chunk_len));
    }

    // Instance the collision checking threads
    for i in 0..COLLISION_THREAD_COUNT {
        let system_clone = Arc::clone(&particle_system_mut);

        collision_pool.execute(move || collision_thread_main(system_clone));
    }

    pool.join();
    collision_pool.join();

    // Bring particles back to the main thread
    let system = particle_system_mut.lock().unwrap();
    system.debug_print_particles();
}
