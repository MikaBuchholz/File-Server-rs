# About this project

This is a simple local file server for _reading_, _writing_ files - creating or deleting files and directories

## **How to use this**:

## Server start

---

When the server starts it will allways try to and create the **root** folder as the genesis of the server

---

## What **you** have to send:

- Template for a request

  ```json
  {
    "instr": "<KnownInstruction>",
    "path": "<PathToWhatever>",
    "content": "<Optional>"
  }
  ```

  > **Hint**: Dont use the <> in the actual request

---

| Instruction |                  Description                  |      Parameters      |
| ----------- | :-------------------------------------------: | :------------------: |
| WRTFILE     |       Write _content_ to file at _path_       | instr, path, content |
| DELFILE     |             Delete file at _path_             |     instr, path      |
| DELDIR      |          Delete directory at _path_           |     instr, path      |
| READDIR     |   Get all children of a directory of _path_   |     instr, path      |
| READFILE    |        Read contents of file at _path_        |     instr, path      |
| CRTFILE     |  Creates a file at _path_ eg. ./root/my.txt   |     instr, path      |
| CRTDIR      | Creates a directory at _path_ eg. ./root/test |     instr, path      |

> **Hint**: path can never be outside of **root** for reason of safety

Example request to create **test.txt** in the **root** folder

```json
{
  "instr": "CRTFILE",
  "path": "./root/test.txt"
}
```

> **Hint**: While _content_ is not a required parameter here, its not illegal to send it regardless

```json
{
  "instr": "CRTFILE",
  "path": "./root/test.txt",
  "content": "Hello from client 😂"
}
```

Will produce the same result as the first request
