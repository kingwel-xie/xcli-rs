use log::{info, LevelFilter};
use xcli::*;

fn main() {
    env_logger::init();

    info!("cli started");

    let mut app = App::new("xCLI")
        .version("v0.1")
        .author("kingwel.xie@139.com");

    app.add_subcommand(
        Command::new("test1")
            .about("controls testing features")
            .action(|_app, _| -> XcliResult {
                println!("tested");
                log::set_max_level(LevelFilter::Info);
                Ok(CmdExeCode::Ok)
            }),
    );

    app.add_subcommand(
        Command::new("mismatch")
            .about("controls testing features")
            .action(|_app, args| -> XcliResult {
                let a = args.iter().map(|&a|a.to_string()).collect();
                Err(XcliError::MismatchArgument(a))
            }),
    );

    app.add_subcommand(
        Command::new("bad")
            .about("controls testing features")
            .action(|_app, _args| -> XcliResult {
                Err(XcliError::BadArgument("bad".into()))
            }),
    );

    app.add_subcommand(
        Command::new("missing")
            .about("controls testing features")
            .action(|_app, _args| -> XcliResult {
                Err(XcliError::MissingArgument)
            }),
    );

    app.run();
}
