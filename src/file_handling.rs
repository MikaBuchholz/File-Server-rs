use std::{fs, io, path::Path};

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

pub fn init_root() {
    match fs::create_dir("./root") {
        Ok(_) => {}
        Err(_) => {}
    }
}

pub async fn execute_instruction(
    instr: &str,
    path: &str,
    socket: &mut tokio::net::TcpStream,
) -> io::Result<()> {
    let response: &str = match instr {
        CREATE_DIR_INSTR => match create_path_to(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => "HTTP/1.1 200 OK\r\n\r\nCreated directory!",
                FileResult::DoesNotExist => "HTTP/1.1 200 OK\r\n\r\nUnreachable",
                FileResult::Exists => "HTTP/1.1 200 OK\r\n\r\nDirectory already exists!",
            },
            Err(_) => "HTTP/1.1 400 Bad\r\n\r\nCargo was valid. Failed to create directory!",
        },
        CREATE_FILE_INSTR => match create_file_at(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => "HTTP/1.1 200 OK\r\n\r\nCreated file!",
                FileResult::DoesNotExist => "HTTP/1.1 200 OK\r\n\r\nUnreachable",
                FileResult::Exists => "HTTP/1.1 200 OK\r\n\r\nFile already exists!",
            },
            Err(_) => "HTTP/1.1 400 Bad\r\n\r\nCargo was valid. Failed to create file!",
        },
        DELETE_FILE_INSTR => match delete_file(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => "HTTP/1.1 200 OK\r\n\r\nDeleted file!",
                FileResult::DoesNotExist => "HTTP/1.1 200 OK\r\n\r\nFile does not exist!",
                FileResult::Exists => "HTTP/1.1 200 OK\r\n\r\nUnreachable",
            },
            Err(_) => "HTTP/1.1 400 Bad\r\n\r\nPayload was valid. Failed to delete file!",
        },
        DELETE_DIR_INSTR => match delete_dir(path) {
            Ok(file_res) => match file_res {
                FileResult::Success => "HTTP/1.1 200 OK\r\n\r\nDeleted directory!",
                FileResult::DoesNotExist => "HTTP/1.1 200 OK\r\n\r\nDirectory does not exist!",
                FileResult::Exists => "HTTP/1.1 200 OK\r\n\r\nUnreachable",
            },
            Err(_) => "HTTP/1.1 400 Bad\r\n\r\nPayload was valid. Failed to delete file!",
        },
        &_ => "HTTP/1.1 400 Bad\r\n\r\nUnknown instruction\r\n\r\n",
    };

    send_response(socket, &response).await?;

    Ok(())
}
