use std::{
    fs,
    io::{self, BufReader, Read},
    path::Path, collections::HashMap,
};

use crate::server::send_response;

const CREATE_DIR_INSTR: &str = "CRTDIR";
const CREATE_FILE_INSTR: &str = "CRTFILE";
const DELETE_FILE_INSTR: &str = "DELFILE";
const DELETE_DIR_INSTR: &str = "DELDIR";
const READ_DIR_INSTR: &str = "READDIR";
const READ_FILE_INSTR: &str = "READFILE";

#[derive(PartialEq, Debug)]
enum FileResult {
    Success,
    Exists,
    DoesNotExist,
    Error,
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
        return Ok(FileResult::Exists);
    }

    fs::create_dir_all(path)?;

    Ok(FileResult::Success)
}

fn create_file_at(path: &str) -> std::io::Result<FileResult> {
    if file_exists(&path) {
        return Ok(FileResult::Exists);
    }

    fs::File::create(&path)?;

    Ok(FileResult::Success)
}

fn delete_file(path: &str) -> std::io::Result<FileResult> {
    if !file_exists(&path) {
        return Ok(FileResult::DoesNotExist);
    }
    fs::remove_file(&path)?;
    Ok(FileResult::Success)
}

fn delete_dir(path: &str) -> std::io::Result<FileResult> {
    if !dir_exists(&path) {
        return Ok(FileResult::DoesNotExist);
    }
    fs::remove_dir_all(path)?;
    Ok(FileResult::Success)
}

async fn read_file(path: &str) -> Result<(String, usize), FileResult> {
    if !file_exists(&path) {
        return Err(FileResult::DoesNotExist);
    }

    let file = fs::File::open(path).unwrap();
    let mut buf_reader = BufReader::new(file);
    let mut buffer = String::new();

    let length = buf_reader.read_to_string(&mut buffer);

    if length.is_err() {
        return Err(FileResult::Error);
    }

    Ok((buffer, length.unwrap()))
}

async fn read_dir(path: &str) -> Result<(String, usize), FileResult> {
    if !dir_exists(&path) {
        return Err(FileResult::DoesNotExist);
    }

    //let origin: String = format!("{path}/");
    let mut dir_string = String::new();

    match fs::read_dir(path) {
        Ok(dir_content) => {
            for wrapped_content in dir_content {
                match wrapped_content {
                    Ok(unwrapped_content) => match unwrapped_content.file_name().into_string() {
                        Ok(ok_content) => {
                            let cur_dir: String = format!("{ok_content}/");
                            dir_string.push_str(&cur_dir);
                        }
                        Err(_) => return Err(FileResult::Error),
                    },
                    Err(_) => return Err(FileResult::Error),
                }
            }
        }
        Err(_) => return Err(FileResult::Error),
    };

    Ok((dir_string.clone(), dir_string.capacity()))
}

pub fn init_root() {
    match fs::create_dir("./root") {
        Ok(_) => {}
        Err(_) => {}
    }
}

pub fn build_json_response(content: String, len: usize) -> String {
    format!(
        "HTTP/1.1 200 OK\r\n\r\nContent-Type: application/json\r\nAccept: */*\r\nContent-Length: {len}\r\n\r\n{{\"content\": \"{content}\"}}",
        len=len,
        content=content.replace("\"", "\\\"")
     )
}

pub async fn execute_instruction(
    instr: &str,
    path: &str,
    socket: &mut tokio::net::TcpStream,
    cache: &mut HashMap<(String, String), (String, usize)>
) -> io::Result<()> {
    let response: String = match instr {
        CREATE_DIR_INSTR => match create_path_to(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => "HTTP/1.1 200 OK\r\n\r\nCreated directory!".into(),
                FileResult::Exists => "HTTP/1.1 200 OK\r\n\r\nDirectory already exists!".into(),
                _ => "HTTP/1.1 200 OK\r\n\r\nUnreachable. CD:_".into(),
            },
            Err(_) => "HTTP/1.1 400 Bad\r\n\r\nCargo was valid. Failed to create directory!".into(),
        },
        CREATE_FILE_INSTR => match create_file_at(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => "HTTP/1.1 200 OK\r\n\r\nCreated file!".into(),
                FileResult::Exists => "HTTP/1.1 200 OK\r\n\r\nFile already exists!".into(),
                _ => "HTTP/1.1 200 OK\r\n\r\nUnreachable. CF:_".into(),
            },
            Err(_) => "HTTP/1.1 400 Bad\r\n\r\nCargo was valid. Failed to create file!".into(),
        },
        DELETE_FILE_INSTR => match delete_file(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => "HTTP/1.1 200 OK\r\n\r\nDeleted file!".into(),
                FileResult::DoesNotExist => "HTTP/1.1 200 OK\r\n\r\nFile does not exist!".into(),
                _ => "HTTP/1.1 200 OK\r\n\r\nUnreachable DF:_".into(),
            },
            Err(_) => {
                "HTTP/1.1 400 Bad\r\n\r\nRequest header seems to be valid. Failed to delete file!"
                    .into()
            }
        },
        DELETE_DIR_INSTR => match delete_dir(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => "HTTP/1.1 200 OK\r\n\r\nDeleted directory!".into(),
                FileResult::DoesNotExist => {
                    "HTTP/1.1 200 OK\r\n\r\nDirectory does not exist!".into()
                }
                _ => "HTTP/1.1 200 OK\r\n\r\nUnreachable. DD:_".into(),
            },
            Err(_) => {
                "HTTP/1.1 400 Bad\r\n\r\nRequest header seems to be valid. Failed to delete file!"
                    .into()
            }
        },
        READ_FILE_INSTR => match read_file(path).await {
            Ok((content, len)) => {cache.insert((instr.into(), path.into()), (content.clone(), len)); build_json_response(content, len)},
            Err(e) => match e {
                FileResult::DoesNotExist => {
                    "HTTP/1.1 400 Bad\r\n\r\nRequested file does not exist!".into()
                }
                FileResult::Error => {
                    "HTTP/1.1 400 Bad\r\n\r\nRequest header seems to be valid. Failed to read file!"
                        .into()
                }
                _ => "HTTP/1.1 200 OK\r\n\r\nUnreachable. RF:_".into(),
            },
        },

        READ_DIR_INSTR => match read_dir(path).await {
            Ok((content, len)) => {cache.insert((instr.into(), path.into()), (content.clone(), len)); build_json_response(content, len)},
            Err(e) => match e {
                FileResult::DoesNotExist => {
                    "HTTP/1.1 400 Bad\r\n\r\nRequested directory does not exist!".into()
                }
                FileResult::Error => {
                    "HTTP/1.1 400 Bad\r\n\r\nRequest header seems to be valid. Failed to read directory!"
                        .into()
                }
                _ => "HTTP/1.1 200 OK\r\n\r\nUnreachable. RD:_".into(),
            },
            
        },

        &_ => "HTTP/1.1 400 Bad\r\n\r\nUnknown instruction\r\n\r\n".into(),
    };

    send_response(socket, response).await?;

    Ok(())
}
