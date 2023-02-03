use std::{
    fs::{self, OpenOptions},
    io::{BufReader, Read, Write},
    path::Path,
};

use crate::server::{bad_400, ok_200, send_response};

const CREATE_DIR_INSTR: &str = "CRTDIR";
const CREATE_FILE_INSTR: &str = "CRTFILE";
const DELETE_FILE_INSTR: &str = "DELFILE";
const DELETE_DIR_INSTR: &str = "DELDIR";
const READ_DIR_INSTR: &str = "READDIR";
const READ_FILE_INSTR: &str = "READFILE";
const WRITE_TO_FILE_INSTR: &str = "WRTFILE";

pub const POST_INSTR: &[&str; 2] = &[CREATE_DIR_INSTR, CREATE_DIR_INSTR];
pub const DELETE_INSTR: &[&str; 2] = &[DELETE_DIR_INSTR, DELETE_FILE_INSTR];
pub const GET_INSTR: &[&str; 2] = &[READ_DIR_INSTR, READ_FILE_INSTR];
pub const PUT_INSTR: &[&str; 1] = &[WRITE_TO_FILE_INSTR];

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

async fn write_to_file(path: &str, content: String) -> Result<(), FileResult> {
    if !file_exists(&path) {
        return Err(FileResult::DoesNotExist);
    }

    match OpenOptions::new().append(true).open(path) {
        Ok(mut file) => match file.write_all(content.as_bytes()) {
            Ok(_) => {}
            Err(_) => return Err(FileResult::Error),
        },
        Err(_) => return Err(FileResult::Error),
    }

    Ok(())
}

async fn read_dir(path: &str) -> Result<(String, usize), FileResult> {
    if !dir_exists(&path) {
        return Err(FileResult::DoesNotExist);
    }

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

fn valid_header(instr: &str, path: &str, text: &Option<String>) -> String {
    match text {
        Some(txt) => 
            format!("Request header was valid. Still failed action: `{instr}` on `{path}`\nInstruction: `{instr}`\nPath: `{path}`\nContent: `{txt}`"),
        None => 
            format!("Request header was valid. Still failed action: `{instr}` on `{path}`\nInstruction: `{instr}`\nPath: `{path}`\n")
    }
}

pub async fn execute_instruction(
    instr: &str,
    path: &str,
    text: &Option<String>,
    socket: &mut tokio::net::TcpStream,
) {
    let response: String = match instr {
        CREATE_DIR_INSTR => match create_path_to(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => ok_200(format!("Created directory at: `{path}`")),
                FileResult::Exists => ok_200(format!("Directory `{path}` already exists.")),
                _ => bad_400("Unreachable. CD:_"),
            },
            Err(_) => ok_200(valid_header(instr, path, text)),
        },
        CREATE_FILE_INSTR => match create_file_at(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => ok_200(format!("Created file at: `{path}`")),
                FileResult::Exists => ok_200(format!("File `{path}` already exists.")),
                _ => bad_400("Unreachable. CF:_"),
            },
            Err(_) => ok_200(valid_header(instr, path, text)),
        },
        DELETE_FILE_INSTR => match delete_file(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => ok_200(format!("Deleted file at: `{path}`")),
                FileResult::DoesNotExist => {
                    ok_200(format!("File at supplied path: `{path}` does not exists."))
                }
                _ => bad_400("Unreachable DF:_"),
            },
            Err(_) => ok_200(valid_header(instr, path, text)),
        },
        DELETE_DIR_INSTR => match delete_dir(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => ok_200(format!("Deleted directory at: `{path}`")),
                FileResult::DoesNotExist => ok_200(format!(
                    "Directory at supplied path: `{path}` does not exists."
                )),
                _ => bad_400("Unreachable. DD:_"),
            },
            Err(_) => ok_200(valid_header(instr, path, text)),
        },
        READ_FILE_INSTR => match read_file(path).await {
            Ok((content, len)) => build_json_response(content, len),
            Err(e) => match e {
                FileResult::DoesNotExist => ok_200(format!(
                    "Can not read file at: `{path}` because it does not exist"
                )),
                FileResult::Error => ok_200(valid_header(instr, path, text)),
                _ => bad_400("Unreachable. RF:_"),
            },
        },

        READ_DIR_INSTR => match read_dir(path).await {
            Ok((content, len)) => build_json_response(content, len),
            Err(e) => match e {
                FileResult::DoesNotExist => ok_200(format!(
                    "Can not read directory at: `{path}` because it does not exist"
                )),
                FileResult::Error => valid_header(instr, path, text),
                _ => bad_400("Unreachable. RD:_"),
            },
        },

        WRITE_TO_FILE_INSTR => match text {
            Some(txt) => match write_to_file(path, txt.to_string()).await {
                Ok(_) => ok_200(format!(
                    "Text: `{txt}` successfuly written to file at `{path}`"
                )),
                Err(e) => match e {
                    FileResult::DoesNotExist => ok_200(format!(
                        "Can not write to file at: `{path}` because it does not exist"
                    )),
                    FileResult::Error => ok_200(valid_header(instr, path, text)),
                    _ => bad_400("Unreachable. WF:_"),
                },
            },
            None => bad_400(format!(
                "Can not write to `{path}` because not content was provided"
            )),
        },

        &_ => bad_400(format!("Unknown instruction: `{instr}`")),
    };

    send_response(socket, response).await;
}
