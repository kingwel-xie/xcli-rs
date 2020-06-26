use std::cell::RefCell;
use std::rc::Rc;
use std::fmt::Debug;
use std::ops::Add;

use log::{debug, info, LevelFilter};

use rustyline::{Editor, EditMode};
use rustyline::config::Configurer;
use rustyline::config::CompletionType;
use rustyline::completion::Completer;

use rustyline_derive::{Helper, Hinter, Highlighter, Validator};
use std::io::{BufWriter, Write};

/// action for CLI commands
type CmdAction = fn(&App, &Vec<&str>) -> CmdExeCode;


/// return code of Command action
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CmdExeCode {
    /// cmd execution ok
    Ok,
    /// exit CLI, by cmd or CTL+C
    Exit,
    /// bad cmd syntax, show help string
    BadSyntax,
    /// bad cmd arguments, can't parse, show help string
    BadArgument(Option<String>),
}

#[derive(Default)]
pub struct App<'a> {
    pub(crate) name: String,
    pub(crate) version: Option<&'a str>,
    pub(crate) author: Option<&'a str>,
    pub(crate) command: Command<'a>,
    pub(crate) rl: Option<Rc<RefCell<Editor<PrefixCompleter>>>>,
}


#[derive(Default, Clone)]
pub struct Command<'a> {
    pub(crate) name: String,
    pub(crate) about: Option<&'a str>,
    pub(crate) aliases: Option<Vec<(&'a str, bool)>>, // (name, visible)
    pub(crate) usage_str: Option<&'a str>,
    pub(crate) usage: Option<&'a str>,
    pub(crate) help_str: Option<&'a str>,
    //pub(crate) args: MKeyMap<'a>,
    pub(crate) subcommands: Vec<Command<'a>>,
    pub(crate) action: Option<CmdAction>,
}

impl<'a> App<'a> {
    pub fn new<S: Into<String>>(n: S) -> Self {
        // note we set the name of roor command to "", len = 0
        let builtin_cmds =  Command::new("")
            .about("Interactive CLI")
            .subcommand(Command::new("tree")
                .about("prints the whole command tree")
                .usage("tree")
                .action(|app, _| -> CmdExeCode {
                    app.show_tree();
                    CmdExeCode::Ok
                })
            )
            .subcommand(Command::new("mode")
                .about("manages the line editor mode, vi/emcas")
                .usage("mode [vi|emacs]")
                .action(cli_mode)
            )
            .subcommand(Command::new("log")
                .about("manages log level filter")
                .usage("log [off|error|warn|info|debug|trace]")
                .action(cli_log)
            )
            .subcommand(Command::new("help")
                .about("displays help information")
                .usage("help [command]")
                .action(cli_help)
            )
            .subcommand(Command::new("test")
                .about("controls testing features")
                // .action(|_app, _| -> CmdExeCode {
                //     println!("tested");
                //     log::set_max_level(LevelFilter::Info);
                //     CmdExeCode::Ok
                // })
                .subcommand(Command::new("c1")
                    .about("controls testing features")
                    .action(|_app, _| -> CmdExeCode {
                        println!("c1 tested");
                        CmdExeCode::Ok
                    }))
            )
            .subcommand(Command::new("exit")
                .about("quits CLI and exits to shell")
                .action(|_, _| -> CmdExeCode {
                    CmdExeCode::Exit
                }))
            .subcommand(Command::new("version")
                .about("shows version information")
                .action(|app, _| -> CmdExeCode {
                    println!("{}\n{}\n{}\n", app.get_name(), app.get_author(), app.get_version());
                    CmdExeCode::Ok
                }))
            ;

        App {
            name: n.into(),
            command: builtin_cmds,
            ..Default::default()
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_author(&self) -> &str {
        self.author.unwrap_or("")
    }
    pub fn get_version(&self) -> &str {
        self.version.unwrap_or("")
    }

    pub fn author<S: Into<&'a str>>(mut self, author: S) -> Self {
        self.author = Some(author.into());
        self
    }
    pub fn version<S: Into<&'a str>>(mut self, ver: S) -> Self {
        self.version = Some(ver.into());
        self
    }
    /// append a sub tree of commands
    pub fn add_subcommand(&mut self, subcmd: Command<'a>) {
        self.command.subcommands.push(subcmd);
    }

    pub fn show_tree(&self) {
        self.rl.as_ref().unwrap().borrow().helper().unwrap().print_tree("");
        // self.command.for_each("", &mut|c, path| {
        //     println!("{} - {}", path, c.name)
        // });
    }

    fn _run(&self, args: Vec<&str>) -> CmdExeCode {
        self.command.run_sub(&self, &args)
    }

    pub fn run(mut self) {

        info!("starting CLI loop...");

        // `()` can be used when no completer is required
        let rl = Rc::new(RefCell::new(Editor::<PrefixCompleter>::new()));
        rl.borrow_mut().set_completion_type(CompletionType::List);
        rl.borrow_mut().set_helper(Some(PrefixCompleter::new(&self.command)));

        if rl.borrow_mut().load_history("history.txt").is_err() {
            println!("No previous history.");
        }

        // set rl.clone to App
        self.rl = Some(rl.clone());

        loop {
            let readline = rl.borrow_mut().readline("# ");
            let line = match readline {
                Ok(line) => {
                    rl.borrow_mut().add_history_entry(line.as_str());
                    debug!("Line: {}", line);
                    line
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                    let l = rl.borrow_mut().readline("Do you realy want to quit? [Y/n]").unwrap_or("n".parse().unwrap());
                    match l.as_str() {
                        "Y" | "y" => {
                            break
                        },
                        _ => "".parse().unwrap(),
                    }
                }
            };

            let args :Vec<_> = line.split_ascii_whitespace().collect();

            // for no any args, do nothing but to continue loop
            if !args.is_empty() {
                match self._run(args) {
                    CmdExeCode::Exit => break,  // std::process::exit(0)
                    //CmdExeCode::BadArgument => println!("Bad argument!"),
                    _ => {},
                }
            }
        }
        rl.borrow_mut().save_history("history.txt").unwrap();
    }
}

impl<'a> Command<'a> {
    pub fn new<S: Into<String>>(n: S) -> Self {
        Command {
            name: n.into(),
            ..Default::default()
        }
    }
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn action(mut self, action: CmdAction) -> Self {
        self.action = Some(action);
        self
    }
    pub fn about<S: Into<&'a str>>(mut self, about: S) -> Self {
        self.about = Some(about.into());
        self
    }
    pub fn usage<S: Into<&'a str>>(mut self, about: S) -> Self {
        self.usage = Some(about.into());
        self
    }
    pub fn get_subcommands(&self) -> &[Command<'a>] {
        &self.subcommands
    }
    pub fn subcommand(mut self, subcmd: Command<'a>) -> Self {
        self.subcommands.push(subcmd);
        self
    }
    pub fn subcommands<I>(mut self, subcmds: I) -> Self
        where
            I: IntoIterator<Item = Command<'a>>,
    {
        for subcmd in subcmds {
            self.subcommands.push(subcmd);
        }
        self
    }

    /// show help/usage message for command
    pub fn show_command_help(&self) {
        println!("Command:     {}\nUsage:       {}\nDescription: {}", self.name,
                 self.usage.unwrap_or(self.name.as_ref()), self.about.unwrap_or(""));
    }

    /// show help message for command and its subs
    pub fn show_subcommand_help(&self) {
        for cmd in &self.subcommands {
            println!("{:12}: {:14}", cmd.name, cmd.usage.unwrap_or(cmd.name.as_ref()))
        }
    }

    /// locate the sub command by the args given
    pub fn locate_subcommand(&self, args: &Vec<&str>) -> Option<&Command> {
        if !args.is_empty()  {
            if let Some(found) = self.subcommands.iter().find(|&c| c.name.as_str() == args[0]) {
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
    pub fn run_sub(&self, app: &App, args: &Vec<&str>) -> CmdExeCode {
        if !args.is_empty()  {
            for cmd in &self.subcommands {
                if args[0] == cmd.name {
                    return cmd.run_sub(app, args[1..].to_vec().as_ref());
                }
            }
        }

        // hit an action
        if let Some(action) = self.action {
            debug!("action for {}, arg={:?}", self.name, args);
            let ret= action(app, &args);
            match ret {
                CmdExeCode::BadArgument(ref err) => {
                    if err.is_some() {
                        println!("Bad argument : '{}'", err.as_ref().unwrap());
                    } else {
                        println!("Missing argument");
                    }
                    self.show_command_help();
                },
                CmdExeCode::BadSyntax => {
                    println!("Bad syntax : {:?}", args);
                    self.show_command_help();
                }
                _ => {}
            }

            return ret;
        } else {
            // no action defined, print error message if there are some unrecognized args
            // otherwise, show help message for this command
            if args.len() > 0 {
                debug!("command without action, but with some args {:?}", args);
                println!("unknown command {} run_cmd {:?}", self.name, args)
            } else {
                debug!("command with no action defined");
                self.show_command_help();
                self.show_subcommand_help();
            }
        }

        CmdExeCode::Ok
    }

    pub fn for_each<F>(&self, path: &str, f: &mut F) where F: FnMut(&Self, &str) {
        f(&self, path);
        for a in self.get_subcommands() {
            a.for_each(format!("{}/{}", path, a.name).as_str(), f);
        }
    }
}


/// A `PrefixCompleter` for subcommands
#[derive(Helper, Hinter, Validator, Highlighter)]
pub struct PrefixCompleter {
    tree: PrefixNode
}

#[derive(Debug, Clone)]
pub struct PrefixNode {
    name:     String,
    children: Vec<PrefixNode>,
}

impl PrefixNode {
    fn new(cmd :&Command) -> PrefixNode {
        PrefixNode {
            // append a space to the cmd name
            name: cmd.name.clone().add(" "),
            children: vec![]
        }
    }
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

    pub fn _complete_cmd(node: &PrefixNode, line: &str, pos: usize) -> Vec<String> {
        debug!("cli to complete {} for node {}", line, node.name);
        let line = line[..pos].trim_start();
        let mut go_next = false;

        let mut new_line:Vec<String> = vec![];
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
            } else {
                if child.name.starts_with(line) {
                    new_line.push(child.name[line.len()..].to_string());
                    offset = line.len();
                    next_node = Some(child);
                }
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
        return new_line;
    }

    fn print_tree(&self, prefix: &str) {
        let s:Vec<u8> = vec![];
        let mut writer = BufWriter::new(s);
        let _ = PrefixCompleter::_print_tree(&self.tree, prefix, 0, &mut writer);
        println!("{}", String::from_utf8_lossy(writer.buffer()));
    }

    fn _print_tree(node: &PrefixNode, prefix: &str, level:u32, buf: &mut BufWriter<Vec<u8>>) -> std::io::Result<()> {
        let mut level = level;
        if node.name.len() > 0 {
            write!(buf, "{}", prefix)?;
            if level > 0 {
                write!(buf, "{}{} ", "├", "─".repeat((level as usize *4)-2))?;
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

    fn complete(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> rustyline::Result<(usize, Vec<String>)> {
        self.complete_cmd(line, pos)
    }
}


fn cli_help(app: &App, args: &Vec<&str>) -> CmdExeCode {
    if args.is_empty() {
        app.command.show_subcommand_help();
    } else {
        if let Some(cmd) = app.command.locate_subcommand(args) {
            cmd.show_command_help();
        } else {
            println!("unknown command {} run_cmd {:?}", app.name, args)
        }
    }
    CmdExeCode::Ok
}

fn cli_log(_app: &App, args: &Vec<&str>) -> CmdExeCode {
    match args.len() {
        0 => {
            println!("Global log level is: {}", log::max_level().to_string());
        },
        1 => {
            match args[0].parse::<LevelFilter>() {
                Ok(level) => log::set_max_level(level),
                Err(err) => return CmdExeCode::BadArgument(Some(format!("{}, {}", args[0], err))),
            }

        },
        _ => return CmdExeCode::BadSyntax,
    }

    CmdExeCode::Ok
}

fn cli_mode(app: &App, args: &Vec<&str>) -> CmdExeCode {
    match args.len() {
        0 => {
            let mode = app.rl.as_ref().unwrap().borrow_mut().config_mut().edit_mode();
            let mode_str = if mode == EditMode::Vi { "Vi"} else { "Emacs" };
            println!("Current edit mode is: {}", mode_str);
        },
        1 => {
            match args[0].to_lowercase().as_ref() {
                "vi" => app.rl.as_ref().unwrap().borrow_mut().set_edit_mode(EditMode::Vi),
                "emacs" => app.rl.as_ref().unwrap().borrow_mut().set_edit_mode(EditMode::Emacs),
                bad => return CmdExeCode::BadArgument(Some(bad.to_string())),
            }
        },
        _ => return CmdExeCode::BadSyntax,
    }

    CmdExeCode::Ok
}