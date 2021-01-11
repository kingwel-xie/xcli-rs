# xcli-rs

A CLI implementation in Rust that is based on Rustyline.

**Supported Platforms**
* Unix
* Windows
   * cmd.exe
   * Powershell
* MacOS (not tested yet)

**Note**:
* " quoted argument is not supported
* No prompt is shown when running on non-tty device. Need a simple tweak on Rustyline...

## Example
```no_run
use xcli::*;

let mut app = App::new("xCLI")
    .version("v0.1")
    .author("kingwel.xie@139.com");

app.add_subcommand(Command::new("qwert")
    .about("controls testing features")
    .usage("qwert")
    .action(|_app, _actions| -> XcliResult {
        println!("qwert tested");
        Ok(CmdExeCode::Ok)
    }));

app.run();
```

## crates.io
You can use this package in your project by adding the following
to your `Cargo.toml`:

```toml
[dependencies]
xcli = "0.5.1"
```

## ChangeLog

- 2020.12.31, v0.5.0 API changes. 
    + Allow a user data to be registered into xCli APP, and it can be retrieved later by user specified CLI commands
    + Refactor xCliError to take more error types. 
    + add_subcommand_with_userdata() for attaching userdata to CLI sub commands
    + example updated to reflect the new APIs 

- 2021.1.11, v0.5.1
    + Command alias: command can have a short name
        ```no_run
        tree            : tree
        mode            : mode [vi|emacs]
        log, l          : log [off|error|warn|info|debug|trace]
        help, h         : help [command]
        exit            : exit
        version, v      : version
        ```
      