extern crate serde_json;

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, DirEntry, File};
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize, Deserialize)]
pub struct Job {
    source_path: String,
    source_sha256sum: String,
    destination_path: String,
    destination_sha256sum: String,
}

#[derive(Serialize, Deserialize)]
pub struct BatchJob {
    source_dir: String,
    destination_dir: String,
    rules: HashMap<String, String>,
    jobs: Vec<Job>,
}

impl BatchJob {
    pub fn new(source_dir: String, destination_dir: String) -> BatchJob {
        let mut bj = BatchJob {
            source_dir: source_dir,
            destination_dir: destination_dir,
            rules: HashMap::new(),
            jobs: vec![],
        };

        bj.rules.insert(
            String::from("mp4"),
            String::from("ffmpeg -i $file_path -c:v libx264 -preset slow"),
        );

        bj
    }

    pub fn init(&mut self) -> Result<(), Box<Error>> {
        let mut lst = vec![];

        // flatten
        {
            let mut add_it = |x: &DirEntry| {
                lst.push(x.path());
            };

            self.visit_dirs(Path::new(&self.source_dir.trim()), &mut add_it)?;
        }

        println!("scanned {} files", lst.len());

        for file in lst.iter() {
            let source_path = file.to_str().unwrap();
            println!("processing {}", source_path);
            let mut file = File::open(source_path).expect("Unable to create config file");
            let hash = Sha256::digest_reader(&mut file)?;

            self.jobs.push(Job {
                source_path: String::from(source_path),
                source_sha256sum: String::from(format!("{:x}", hash)),
                destination_path: String::new(),
                destination_sha256sum: String::new(),
            });
        }

        Ok(())
    }

    pub fn load_from_file(dir: &str) -> Result<BatchJob, Box<Error>> {
        let filepath = format!("{}/dbfc.config", dir);
        let mut file = File::open(filepath)?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer)?;
        let bj: BatchJob = serde_json::from_str(&buffer)?;

        Ok(bj)
    }

    pub fn run(&mut self) {
        println!("Showing current BatchJob");
        for rule in self.rules.iter() {
            println!("{:?}", rule);
        }
    }

    fn visit_dirs(&self, dir: &Path, cb: &mut FnMut(&DirEntry)) -> Result<(), Box<Error>> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    self.visit_dirs(&path, cb)?;
                } else {
                    cb(&entry);
                }
            }
        }
        Ok(())
    }

    fn convert_video(&self, source_filepath: &Path, output_dir: &str) {
        let mut out_file = PathBuf::new();
        out_file.push(output_dir);
        out_file.push(source_filepath.file_stem().unwrap());
        out_file.set_extension("mp4");

        let mut output = Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("warning")
            .arg("-i")
            .arg(source_filepath)
            .arg("-y")
            .arg("-c:v")
            .arg("libx264")
            .arg("-preset")
            .arg("ultrafast")
            .arg("-framerate")
            .arg("30")
            .arg("-vsync")
            .arg("1")
            .arg("-crf")
            .arg("20")
            .arg(out_file.as_path())
            .spawn()
            .unwrap_or_else(|e| panic!("failed to execute process: {}", e));

        output.wait().unwrap();
    }
}
