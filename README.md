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
xcli = "0.4"
```

## ChangeLog

- 2020.12.31, v0.4.2 API changes. Allow a user data to be registered into xCli APP, and it can be retrieved later by user specified CLI commands. Refactor xCliError to take more error type. 