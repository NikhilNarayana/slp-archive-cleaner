use peppi::game::PlayerType;
use peppi::io::slippi::read;
use std::io::{Read, Write, stdin, stdout};
use std::ops::Sub;
use std::path::PathBuf;
use std::{fs, io};
use walkdir::WalkDir;

// @folder_name for sorting reasons
const CPU_OUTPUT_FOLDER_NAME: &str = "@cpu_games";
const HANDWARMERS_OUTPUT_FOLDER_NAME: &str = "@handwarmers";
/// assume that any game with less damage done than this value is a handwarmer
const MINIMUM_TOURNAMENT_PERCENT: f32 = 100.0;

/// walk down the given directory, or assume cwd, and collect all slp file paths in a vec
fn get_all_slps_paths(root_path: Option<String>) -> Vec<PathBuf> {
    WalkDir::new(root_path.or(Some(".".to_string())).unwrap())
        .follow_links(true)
        .into_iter()
        .filter(|f| f.is_ok())
        .map(|f| f.unwrap().into_path())
        .filter(|f| {
            f.is_file()
                && f.extension().unwrap() == "slp"
                && !f.components().any(|f| {
                    let comp = f.as_os_str();
                    comp == CPU_OUTPUT_FOLDER_NAME || comp == HANDWARMERS_OUTPUT_FOLDER_NAME
                })
        })
        .collect()
}

/// calculate the total damage done by all players in the game
fn calculate_damage_done(game: peppi::game::immutable::Game) -> f32 {
    game.frames // peppi gives us frames in a columnar format
        .ports // ports contains the data for each port for all frames of the game
        .iter()
        .map(|p| {
            // iterate over the frames for each player in groups of 2 to find the change in percent
            p.leader
                .post
                .percent
                .values()
                .to_vec()
                .windows(2)
                .filter_map(|f| match (f.first(), f.last()) {
                    (Some(initial_percent), Some(next_percent)) => {
                        // filter out no change or stock change
                        if initial_percent == next_percent || initial_percent > next_percent {
                            None
                        } else {
                            Some(next_percent.sub(initial_percent))
                        }
                    }
                    _ => None,
                })
                .sum::<f32>()
        })
        .sum::<f32>()
}

fn game_has_cpu_player(game: &peppi::game::immutable::Game) -> bool {
    game.start
        .players
        .iter()
        .any(|f| f.r#type == PlayerType::Cpu)
}

fn process_slps(slp_paths: Vec<PathBuf>) {
    for f in slp_paths {
        let mut r = io::BufReader::new(fs::File::open(&f).unwrap());
        let game = match read(&mut r, None) {
            Ok(game) => game,
            Err(err) => {
                println!("failed to read game: {} [{}]", f.display(), err);
                continue;
            }
        };

        // cpu check
        if game_has_cpu_player(&game) {
            let _ = fs::create_dir_all(CPU_OUTPUT_FOLDER_NAME);
            println!("{}: cpu_match", f.display());
            let mut old_path = PathBuf::new();
            old_path.push(&f);
            let new_path = format!(
                "{}/{}",
                CPU_OUTPUT_FOLDER_NAME,
                old_path.file_name().unwrap().to_str().unwrap()
            );
            match fs::rename(&f, new_path) {
                Ok(_) => (),
                Err(err) => println!("failed to move cpu match: file={} err={}", f.display(), err),
            }
            continue;
        }

        // handwarmer check
        let damage_done = calculate_damage_done(game);
        if damage_done < MINIMUM_TOURNAMENT_PERCENT {
            let _ = fs::create_dir_all(HANDWARMERS_OUTPUT_FOLDER_NAME);
            println!("{}: damage_done = {}", f.display(), damage_done);
            let mut old_path = PathBuf::new();
            old_path.push(&f);
            let new_path = format!(
                "{}/{}",
                HANDWARMERS_OUTPUT_FOLDER_NAME,
                old_path.file_name().unwrap().to_str().unwrap()
            );
            match fs::rename(&f, new_path) {
                Ok(_) => (),
                Err(err) => println!(
                    "failed to move friendlies match: file={} err={}",
                    f.display(),
                    err
                ),
            }
            continue;
        }
    }
}

fn main() {
    let slp_paths = get_all_slps_paths(None);
    process_slps(slp_paths);

    let mut stdout = stdout();
    stdout.write(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read(&mut [0]).unwrap();
}
