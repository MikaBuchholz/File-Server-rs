use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::server::send_response;

const CREATE_DIR_INSTR: &str = "CRTDIR";
const CREATE_FILE_INSTR: &str = "CRTFILE";
const DELETE_FILE_INSTR: &str = "DELFILE";
const DELETE_DIR_INSTR: &str = "DELDIR";

#[derive(PartialEq, Debug)]
enum FileResult {
    CREATED,
    EXISTS,
}

fn dir_exists(dir: &str) -> bool {
    let path = Path::new(dir);
    match path.metadata() {
        Ok(md) => md.is_dir(),
        Err(_) => false,
    }
}

fn file_exists(file: &str) -> bool {
    let path = Path::new(file);
    match path.metadata() {
        Ok(md) => md.is_file(),
        Err(_) => false,
    }
}

pub fn is_inside_root(path: &String) -> bool {
    path.starts_with("./root")
}

fn create_path_to(path: &str) -> std::io::Result<FileResult> {
    if dir_exists(&path) {
        return Ok(FileResult::EXISTS);
    }

    fs::create_dir_all(path)?;

    Ok(FileResult::CREATED)
}

fn create_file_at<'a>(path: &'a str) -> std::io::Result<FileResult> {
    if file_exists(&path) {
        return Ok(FileResult::EXISTS);
    }

    fs::File::create(&path)?;

    Ok(FileResult::CREATED)
}

fn delete_file(path: &Path) -> std::io::Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

fn delete_dir(path: &Path) -> std::io::Result<()> {
    fs::remove_dir_all(path)?;
    Ok(())
}

pub fn init_root() {
    let dir_entries = fs::read_dir(".");

    match dir_entries {
        Ok(entries) => {
            let entry_vec: Result<Vec<PathBuf>, std::io::Error> =
                entries.map(|res| res.map(|e| e.path())).collect();

            match entry_vec {
                Ok(entr_vec) => {
                    if entr_vec.contains(&PathBuf::from("./root")) {
                        return;
                    }

                    match fs::create_dir("./root") {
                        Ok(_) => {}
                        Err(_) => {
                            panic!("Could not create root | Aborting")
                        }
                    }
                }
                Err(_) => {
                    //can not happen
                }
            }
        }
        Err(_) => {
            panic!("Directory could not be read | Aborting")
        }
    }
}

pub async fn execute_instruction(
    instr: &str,
    path: &str,
    socket: &mut tokio::net::TcpStream,
) -> io::Result<()> {
    //TODO make this more compact - helper function - maybe an enum
    let response: &str = match instr {
        CREATE_DIR_INSTR => match create_path_to(path) {
            Ok(file_res) => {
                if file_res == FileResult::CREATED {
                    "HTTP/1.1 200 OK\r\n\r\nCreated directory!";
                }
                if file_res == FileResult::EXISTS {
                    "HTTP/1.1 200 OK\r\n\r\nDirectory already exists!"
                } else {
                    //TODO this is reached sometimes
                    "HTTP/1.1 400 Bad\r\n\r\nNot reachable!"
                }
            }
            Err(_) => "HTTP/1.1 400 Bad\r\n\r\nCargo was valid. Failed to create directory!",
        },
        CREATE_FILE_INSTR => match create_file_at(path) {
            Ok(file_res) => {
                if file_res == FileResult::CREATED {
                    "HTTP/1.1 200 OK\r\n\r\nCreated file!";
                }
                if file_res == FileResult::EXISTS {
                    "HTTP/1.1 200 OK\r\n\r\nFile already exists!"
                } else {
                    //TODO this is reached sometimes
                    "HTTP/1.1 400 Bad\r\n\r\nNot reachable!"
                }
            }
            Err(_) => "HTTP/1.1 400 Bad\r\n\r\nCargo was valid. Failed to create file!",
        },
        DELETE_FILE_INSTR => {
            delete_file(Path::new(path))?;

            //TODO handle case if file does not exist

            "HTTP/1.1 200 OK\r\n\r\nDeleted File!"
        }
        DELETE_DIR_INSTR => {
            delete_dir(Path::new(path))?;

            //TODO handle case if dir does not exist

            "HTTP/1.1 200 OK\r\n\r\nDeleted Dir"
        }
        &_ => "HTTP/1.1 400 Bad\r\n\r\nUnknown instruction\r\n\r\n",
    };

    send_response(socket, &response).await?;

    Ok(())
}
