//! # xcli-rs
//!
//! A CLI implementation in Rust that is based on Rustyline.
//!
//! **Supported Platforms**
//! * Unix
//! * Windows
//! * cmd.exe
//! * Powershell
//! * MacOS (not tested yet)
//!
//! **Note**:
//! * " quoted argument is not supported
//! * No prompt is shown when running on non-tty device. Need a simple tweak on Rustyline...
//!
//! ## Example
//! ```rust
//! use xcli::*;
//!
//!
//! let mut app = App::new("xCLI")
//!     .version("v0.1")
//!     .author("kingwel.xie@139.com");
//!
//! app.add_subcommand(Command::new("qwert")
//!     .about("controls testing features")
//!     .usage("qwert")
//!     .action(|_app, _actions| -> XcliResult {
//!         println!("qwert tested");
//!         Ok(CmdExeCode::Ok)
//!     }));
//!
//! app.run();
//!
//! ```
//!
//! ## crates.io
//! You can use this package in your project by adding the following
//! to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! xcli = "0.5"
//! ```

use std::cell::RefCell;
use std::fmt::Debug;
use std::ops::Add;
use std::rc::Rc;

use log::{debug, info, LevelFilter};

use rustyline::completion::Completer;
use rustyline::config::CompletionType;
use rustyline::config::Configurer;
use rustyline::{EditMode, Editor};

use rustyline_derive::{Helper, Highlighter, Hinter, Validator};
use std::collections::HashMap;
use std::io::{BufWriter, Write};

use std::result::Result as stdResult;

#[derive(thiserror::Error, Debug)]
pub enum XcliError {
    /// Bad command syntax.
    #[error("Bad syntax")]
    BadSyntax,
    /// The specified handler does not exist.
    #[error("Missing handler: {0} not found")]
    MissingHandler(String),
    /// Missing required argument(s).
    #[error("Missing required argument")]
    MissingArgument,
    /// Bad argument.
    #[error("Bad argument: {0}")]
    BadArgument(String),
    /// Mismatched arguments.
    #[error("Mismatched argument(s): wanted: {0}, actual: {1}")]
    MismatchArgument(usize, usize),
    /// Other error.
    #[error("{0}")]
    Other(String),
}

/// Cmd action execute result
pub type XcliResult = stdResult<CmdExeCode, XcliError>;

/// The action for CLI commands. Note it is a fn pointer instead of Fn(),
/// This avoids an allocation of Box::new.
type CmdAction = fn(&App, &[&str]) -> XcliResult;

/// The return code of Command action.
#[derive(Debug, PartialEq, Eq)]
pub enum CmdExeCode {
    /// cmd execution ok
    Ok,
    /// exit CLI, by cmd or CTL+C
    Exit,
}

type IAny = Box<dyn std::any::Any>;

/// Xcli object
pub struct App<'a> {
    pub(crate) name: String,
    pub(crate) version: Option<&'a str>,
    pub(crate) author: Option<&'a str>,
    pub(crate) tree: Command<'a>,
    pub(crate) rl: Rc<RefCell<Editor<PrefixCompleter>>>,
    pub(crate) handlers: HashMap<String, IAny>,
}

/// Command structure, which describes a command and its action.
#[derive(Default)]
pub struct Command<'a> {
    pub(crate) name: String,
    pub(crate) alias: Option<String>,
    pub(crate) about: Option<&'a str>,
    pub(crate) usage: Option<&'a str>,
    pub(crate) subcommands: Vec<Command<'a>>,
    pub(crate) action: Option<CmdAction>,
}

impl<'a> App<'a> {
    /// Create a new cli instance and return it
    pub fn new<S: Into<String>>(n: S) -> Self {
        // note we set the name of roor command to "", len = 0
        let builtin_cmds = Command::new("")
            .about("Interactive CLI")
            .subcommand(
                Command::new("tree")
                    .about("prints the whole command tree")
                    .usage("tree")
                    .action(|app: &App, _| -> XcliResult {
                        app.show_tree();
                        Ok(CmdExeCode::Ok)
                    }),
            )
            .subcommand(
                Command::new("mode")
                    .about("manages the line editor mode, vi/emcas")
                    .usage("mode [vi|emacs]")
                    .action(cli_mode),
            )
            .subcommand(
                Command::new_with_alias("log", "l")
                    .about("manages log level filter")
                    .usage("log [off|error|warn|info|debug|trace]")
                    .action(cli_log),
            )
            .subcommand(
                Command::new_with_alias("help", "h")
                    .about("displays help information")
                    .usage("help [command]")
                    .action(cli_help),
            )
            .subcommand(
                Command::new("exit")
                    .about("quits CLI and exits to shell")
                    .action(|_, _| -> XcliResult { Ok(CmdExeCode::Exit) }),
            )
            .subcommand(
                Command::new_with_alias("version", "v")
                    .about("shows version information")
                    .action(|app, _| -> XcliResult {
                        println!(
                            "{}\n{}\n{}\n",
                            app.get_name(),
                            app.get_author(),
                            app.get_version()
                        );
                        Ok(CmdExeCode::Ok)
                    }),
            );

        let rl = Rc::new(RefCell::new(Editor::<PrefixCompleter>::new()));

        App {
            name: n.into(),
            version: None,
            author: None,
            tree: builtin_cmds,
            rl,
            handlers: HashMap::default(),
        }
    }

    /// Get the name of this instance
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the author of this instance
    pub fn get_author(&self) -> &str {
        self.author.unwrap_or("")
    }

    /// Get the version of this instance
    pub fn get_version(&self) -> &str {
        self.version.unwrap_or("")
    }

    /// Set the author of this instance
    pub fn author<S: Into<&'a str>>(mut self, author: S) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Get the version of this instance.
    pub fn version<S: Into<&'a str>>(mut self, ver: S) -> Self {
        self.version = Some(ver.into());
        self
    }
    /// Add sub commands to command tree.
    pub fn add_subcommand(&mut self, subcmd: Command<'a>) {
        self.tree.subcommands.push(subcmd);
    }

    /// Add sub commands to commands, with user defined data.
    ///
    /// userdata must be an IAny trait object, which will be associated with the
    /// command name of the subcmd.
    pub fn add_subcommand_with_userdata(&mut self, subcmd: Command<'a>, value: IAny) {
        self.handlers.insert(subcmd.name.clone(), value);
        self.tree.subcommands.push(subcmd);
    }

    /// Show all commands and their subcommands like a tree
    pub fn show_tree(&self) {
        self.rl.borrow().helper().unwrap().print_tree("");
    }

    /// Get handler
    pub fn get_handler<S: Into<String>>(&self, key: S) -> stdResult<&IAny, XcliError> {
        let ks = key.into();
        self.handlers
            .get(&ks)
            .ok_or(XcliError::MissingHandler(ks))
    }

    /// Get the status return by args command
    fn _run(&mut self, args: Vec<&str>) -> XcliResult {
        self.tree.run_sub(&self, &args)
    }

    /// Run the instance
    pub fn run(mut self) {
        info!("starting CLI loop...");

        self.rl
            .borrow_mut()
            .set_completion_type(CompletionType::List);
        self.rl
            .borrow_mut()
            .set_helper(Some(PrefixCompleter::new(&self.tree)));

        if self.rl.borrow_mut().load_history("history.txt").is_err() {
            println!("No previous history.");
        }

        loop {
            let readline = self.rl.borrow_mut().readline("# ");
            let line = match readline {
                Ok(line) => {
                    self.rl.borrow_mut().add_history_entry(line.as_str());
                    debug!("Line: {}", line);
                    line
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    let l = self
                        .rl
                        .borrow_mut()
                        .readline("Do you realy want to quit? [y/N]")
                        .unwrap_or_else(|_| "n".parse().unwrap());
                    match l.as_str() {
                        "Y" | "y" => break,
                        _ => "".to_string(),
                    }
                }
            };

            let args = line.split_ascii_whitespace().collect::<Vec<_>>();

            // skip empty input line
            if !args.is_empty() {
                // for no any args, do nothing but to continue loop
                if let Ok(CmdExeCode::Exit) = self._run(args) {
                    break;
                }
            }
        }
        self.rl.borrow_mut().save_history("history.txt").unwrap();
    }
}

impl<'a> Command<'a> {
    /// Create a command
    pub fn new<S: Into<String>>(n: S) -> Self {
        Command {
            name: n.into(),
            alias: None,
            about: None,
            usage: None,
            subcommands: vec![],
            action: None,
        }
    }

    pub fn new_with_alias<S: Into<String>>(n: S, s: S) -> Self {
        Command {
            name: n.into(),
            alias: Some(s.into()),
            about: None,
            usage: None,
            subcommands: vec![],
            action: None,
        }
    }

    /// Get the name of this command.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the alias name of this command.
    pub fn get_alias(&self) -> &Option<String> {
        &self.alias
    }

    // Get the name description, aka. name+alias of this command.
    fn get_description(&self) -> String {
        let mut s = self.name.clone();
        if let Some(alias) = self.alias.as_ref() {
            s = format!("{}, {} ", s, alias);
        }
        s
    }

    /// Set a CmdAction to this command.
    pub fn action(mut self, action: CmdAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Set a description to this command.
    pub fn about<S: Into<&'a str>>(mut self, about: S) -> Self {
        self.about = Some(about.into());
        self
    }

    /// Set the usage to this command.
    pub fn usage<S: Into<&'a str>>(mut self, about: S) -> Self {
        self.usage = Some(about.into());
        self
    }

    /// Get all subcommands of this command
    pub fn get_subcommands(&self) -> &[Command<'a>] {
        &self.subcommands
    }

    /// Add a subcommand to this command
    pub fn subcommand(mut self, subcmd: Command<'a>) -> Self {
        self.subcommands.push(subcmd);
        self
    }

    /// Add more than one subcommand to this command, the given subcmds implements IntoIterator
    pub fn subcommands<I>(mut self, subcmds: I) -> Self
    where
        I: IntoIterator<Item = Command<'a>>,
    {
        for subcmd in subcmds {
            self.subcommands.push(subcmd);
        }
        self
    }

    /// show usage message for command.
    pub fn show_command_usage(&self) {
        println!("Usage:       {}", self.usage.unwrap_or_else(|| self.name.as_ref()));
    }

    /// show help message for command.
    pub fn show_command_help(&self) {
        println!(
            "Command:     {}\nUsage:       {}\nDescription: {}",
            self.get_description(),
            self.usage.unwrap_or_else(|| self.name.as_ref()),
            self.about.unwrap_or("")
        );
    }

    /// show help message for command and its subs
    pub fn show_subcommand_help(&self) {
        for cmd in &self.subcommands {
            println!(
                "{:16}: {}",
                cmd.get_description(),
                cmd.usage.unwrap_or_else(|| cmd.name.as_ref())
            )
        }
    }

    /// locate the sub command by the args given
    pub fn locate_subcommand(&self, args: &[&str]) -> Option<&Command> {
        if !args.is_empty() {
            if let Some(found) = self
                .subcommands
                .iter()
                .find(|&c| c.name == args[0] || c.alias.as_ref().map_or(false, |a| a == args[0]))
            {
                found.locate_subcommand(args[1..].to_vec().as_ref())
            } else {
                None
            }
        } else {
            Some(self)
        }
    }

    /// run sub commands
    ///
    /// show error message when action not found but with extra args
    ///
    /// show help messages when action not found for the sub command
    ///
    /// execute sub command when action found
    ///
    pub fn run_sub(&self, app: &App, args: &[&str]) -> XcliResult {
        if !args.is_empty() {
            for cmd in &self.subcommands {
                if args[0] == cmd.name || cmd.alias.as_ref().map_or(false, |a| a == args[0]) {
                    return cmd.run_sub(app, args[1..].to_vec().as_ref());
                }
            }
        }

        // hit an action
        if let Some(action) = &self.action {
            debug!("action for {}, arg={:?}", self.name, args);
            let ret = action(app, &args);
            match &ret {
                Err(XcliError::Other(err)) => {
                    println!("{}", err);
                }
                Err(err) => {
                    println!("{}\n", err);
                    self.show_command_usage();
                }
                Ok(_) => {}
            }

            return ret;
        } else {
            // no action defined, print error message if there are some unrecognized args
            // otherwise, show help message for this command
            if !args.is_empty() {
                debug!("command without action, but with some args {:?}", args);
                println!("Unknown command or arguments : {:?}", args)
            } else {
                debug!("command with no action defined");
                self.show_command_help();
                self.show_subcommand_help();
            }
        }

        Ok(CmdExeCode::Ok)
    }

    ///
    pub fn for_each<F>(&self, path: &str, f: &mut F)
    where
        F: FnMut(&Self, &str),
    {
        f(&self, path);
        for a in self.get_subcommands() {
            a.for_each(format!("{}/{}", path, a.name).as_str(), f);
        }
    }
}

/// A `PrefixCompleter` for subcommands
#[derive(Helper, Hinter, Validator, Highlighter)]
pub struct PrefixCompleter {
    tree: PrefixNode,
}

#[derive(Debug, Clone)]
pub struct PrefixNode {
    name: String,
    children: Vec<PrefixNode>,
}

/// Command tree node
impl PrefixNode {
    /// Create a PrefixNode
    fn new(cmd: &Command) -> PrefixNode {
        PrefixNode {
            // append a space to the cmd name
            name: cmd.name.clone().add(" "),
            children: vec![],
        }
    }

    /// Add a child node to this PrefixNode
    fn add_children(&mut self, child: PrefixNode) {
        self.children.push(child);
    }
}

impl PrefixCompleter {
    /// Constructor, take the command tree as input
    pub fn new(cmd_tree: &Command) -> Self {
        let mut prefix_tree = PrefixNode::new(cmd_tree);
        for cmd in &cmd_tree.subcommands {
            PrefixCompleter::generate_cmd_tree(&mut prefix_tree, cmd);
        }

        Self { tree: prefix_tree }
    }

    /// Generate the command tree by cmd and parent
    fn generate_cmd_tree(parent: &mut PrefixNode, cmd: &Command) {
        let mut node = PrefixNode::new(cmd);

        for cmd in &cmd.subcommands {
            PrefixCompleter::generate_cmd_tree(&mut node, cmd);
        }

        debug!("prefix {} added", node.name);
        parent.add_children(node);
    }

    /// Takes the currently edited `line` with the cursor `pos`ition and
    /// returns the start position and the completion candidates for the
    /// partial path to be completed.
    pub fn complete_cmd(&self, line: &str, pos: usize) -> rustyline::Result<(usize, Vec<String>)> {
        debug!("line={} pos={}", line, pos);
        let v = PrefixCompleter::_complete_cmd(&self.tree, line, pos);
        Ok((pos, v))
    }

    /// Get all commands that match the line and pos
    pub fn _complete_cmd(node: &PrefixNode, line: &str, pos: usize) -> Vec<String> {
        debug!("cli to complete {} for node {}", line, node.name);
        let line = line[..pos].trim_start();
        let mut go_next = false;

        let mut new_line: Vec<String> = vec![];
        let mut offset: usize = 0;
        let mut next_node = None;

        //var lineCompleter PrefixCompleterInterface
        for child in &node.children {
            //debug!("try node {}", child.name);
            if line.len() >= child.name.len() {
                if line.starts_with(&child.name) {
                    if line.len() == child.name.len() {
                        // add a fack new_line " "
                        new_line.push(" ".to_string());
                    } else {
                        new_line.push(child.name.to_string());
                    }
                    offset = child.name.len();
                    next_node = Some(child);

                    // may go next level
                    go_next = true;
                }
            } else if child.name.starts_with(line) {
                new_line.push(child.name[line.len()..].to_string());
                offset = line.len();
                next_node = Some(child);
            }
        }

        // more than 1 candidates?
        if new_line.len() != 1 {
            debug!("offset={}, candidates={:?}", offset, new_line);
            return new_line;
        }

        if go_next {
            let line = line[offset..].trim_start();
            return PrefixCompleter::_complete_cmd(next_node.unwrap(), line, line.len());
        }

        debug!("offset={}, nl={:?}", offset, new_line);
        new_line
    }

    /// Print the command tree
    fn print_tree(&self, prefix: &str) {
        let s: Vec<u8> = vec![];
        let mut writer = BufWriter::new(s);
        let _ = PrefixCompleter::_print_tree(&self.tree, prefix, 0, &mut writer);
        println!("{}", String::from_utf8_lossy(writer.buffer()));
    }

    /// Build the command tree recursively
    fn _print_tree(
        node: &PrefixNode,
        prefix: &str,
        level: u32,
        buf: &mut BufWriter<Vec<u8>>,
    ) -> std::io::Result<()> {
        let mut level = level;
        if !node.name.is_empty() {
            write!(buf, "{}", prefix)?;
            if level > 0 {
                write!(buf, "├{}", "─".repeat((level as usize * 4) - 2))?;
            }
            writeln!(buf, "{}", node.name)?;
            level += 1;
        }

        for child in &node.children {
            let _ = PrefixCompleter::_print_tree(child, prefix, level, buf);
        }

        Ok(())
    }
}

impl Completer for PrefixCompleter {
    type Candidate = String;

    /// Complete command
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        self.complete_cmd(line, pos)
    }
}

/// Action of help command
fn cli_help(app: &App, args: &[&str]) -> XcliResult {
    if args.is_empty() {
        app.tree.show_subcommand_help();
    } else if let Some(cmd) = app.tree.locate_subcommand(args) {
        cmd.show_command_help();
    } else {
        println!("Unrecognized command {:?}", args)
    }
    Ok(CmdExeCode::Ok)
}

/// Action of log command
fn cli_log(_app: &App, args: &[&str]) -> XcliResult {
    match args.len() {
        0 => {
            println!("Global log level is: {}", log::max_level().to_string());
        }
        1 => match args[0].parse::<LevelFilter>() {
            Ok(level) => log::set_max_level(level),
            Err(err) => {
                let err = format!("{}, {}", args[0], err);
                return Err(XcliError::BadArgument(err));
            }
        },
        _ => return Err(XcliError::BadSyntax),
    }

    Ok(CmdExeCode::Ok)
}

/// Action of mode command
fn cli_mode(app: &App, args: &[&str]) -> XcliResult {
    match args.len() {
        0 => {
            let mode = app.rl.borrow_mut().config_mut().edit_mode();
            let mode_str = if mode == EditMode::Vi { "Vi" } else { "Emacs" };
            println!("Current edit mode is: {}", mode_str);
        }
        1 => match args[0].to_lowercase().as_ref() {
            "vi" => app.rl.borrow_mut().set_edit_mode(EditMode::Vi),
            "emacs" => app.rl.borrow_mut().set_edit_mode(EditMode::Emacs),
            bad => {
                return {
                    Err(XcliError::BadArgument(bad.into()))
                }
            }
        },
        _ => return Err(XcliError::BadSyntax),
    }

    Ok(CmdExeCode::Ok)
}
