use log::{info, LevelFilter};
use xcli::*;

fn main() {
    env_logger::init();

    info!("cli started");

    let mut app = App::new("xCLI")
        .version("v0.1")
        .author("kingwel.xie@139.com");

    app.add_subcommand(
        Command::new_with_alias("test1", "t1")
            .about("controls testing features")
            .action(|_app, _| -> XcliResult {
                println!("tested");
                log::set_max_level(LevelFilter::Info);
                Ok(CmdExeCode::Ok)
            }),
    );

    app.add_subcommand(
        Command::new_with_alias("mismatch", "mm")
            .about("controls testing features")
            .action(|_app, args| -> XcliResult {
                Err(XcliError::MismatchArgument(10, args.len()))
            }),
    );

    app.add_subcommand(
        Command::new_with_alias("bad", "b")
            .about("controls testing features")
            .action(|_app, _args| -> XcliResult {
                Err(XcliError::BadArgument("bad".into()))
            }),
    );

    app.add_subcommand(
        Command::new_with_alias("missing", "mm")
            .about("controls testing features")
            .action(|_app, _args| -> XcliResult {
                Err(XcliError::MissingArgument)
            }),
    );

    app.add_subcommand_with_userdata(
        Command::new_with_alias("userdata", "ud")
            .about("controls testing features")
            .action(|app, _args| -> XcliResult {
                let data_any = app.get_handler("userdata").unwrap();

                let data = data_any.downcast_ref::<usize>().expect("usize");

                println!("userdata = {}", data);
                Ok(CmdExeCode::Ok)
            }),
        Box::new(100usize)
    );

    app.run();
}
