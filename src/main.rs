use std::io::prelude::*; // parse
use std::net::{TcpStream};
use std::path::Path; // gestion repertoires

use ssh2::{Session, Channel};
use clap::{Arg, App};  // arguments


fn _do_exit(mut channel: Channel, rc: i32) {
    _close_connexion(channel);
    std::process::exit(rc);
}

fn _close_connexion(mut channel: Channel) {
    let closing = channel.wait_close();
}

fn main() -> () {
    let matches = App::new("check_distant_linux")
        .version("0.1.0")
        .author("Jean Gabès <naparuba@gmail.com>")
        .about("Check distant linux with SSH commands, no agent need")
        .arg(Arg::with_name("hostname").short("H").long("hostname").takes_value(true).required(true).help("Hostname to connect to"))
        .arg(Arg::with_name("port").short("p").long("port").takes_value(true).default_value("22").help("SSH port to connect to. Default : 22"))
        .arg(Arg::with_name("ssh-key").short("i").long("ssh-key").takes_value(true).default_value("~/.ssh/id_rsa").help("SSH key file to use. By default will take ~/.ssh/id_rsa."))
        .arg(Arg::with_name("user").short("u").long("user").takes_value(true).default_value("shinken").help("remote use to use. By default shinken."))
        .arg(Arg::with_name("passphrase").short("P").long("passphrase").takes_value(true).default_value("").help("SSH key passphrase. By default will use none"))
        .arg(Arg::with_name("warning").short("w").long("warning").takes_value(true).default_value("0.9,0.9,0.9").help("Warning value for load average, as 3 values, for 1m,5m,15m. Default : 0.9,0.9,0.9"))
        .arg(Arg::with_name("critical").short("c").long("critical").takes_value(true).default_value("1.5,1.5,1.5").help("Critical value for load average, as 3 values, for 1m,5m,15m. Default : 1.5,1.5,1.5"))
        .get_matches();


    let hostname = matches.value_of("hostname").unwrap();
    println!("Hostname: {}", hostname);

    let port_arg = matches.value_of("port");
    let mut port: i32 = 22;
    match port_arg {
        None => println!("No idea what your favorite number is."),
        Some(_port_arg) => {
            match _port_arg.parse::<i32>() {
                Ok(n) => port = n,
                Err(_) => {
                    println!("ERROR: Argument --p/--port must be a number: \"{}\"", _port_arg);
                    std::process::exit(2);
                }
            }
        }
    }



    let addr = format!("{}:{}", hostname, port);
    println!("Connect to {}", addr);
    let tcp = TcpStream::connect(addr).unwrap();
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    //sess.method_pref()
    sess.handshake().unwrap();

    // user, pubkey, privkey, passphrase
    sess.userauth_pubkey_file("root", None, Path::new("/root/.ssh/id_rsa"), None).unwrap();
    //sess.userauth_password("root", "ze").unwrap();
    assert!(sess.authenticated());

    // Create a channel to launch a command
    let mut channel = sess.channel_session().unwrap();

    // Launch it
    channel.exec("echo  \"$(cat /proc/loadavg) $(grep -E '^CPU|^processor' < /proc/cpuinfo | wc -l)\"").unwrap();
    let mut buf = String::new();
    channel.read_to_string(&mut buf).unwrap();
    println!("{}", buf);

    // Parse it
    let mut elts = buf.trim().split(" ");
    let load1 = elts.next().unwrap().parse::<f64>().unwrap();
    let load5 = elts.next().unwrap().parse::<f64>().unwrap();
    let load15 = elts.next().unwrap().parse::<f64>().unwrap();
    let _tmp = elts.next();
    let _tmp2 = elts.next();
    let nb_cpus = elts.next().unwrap().parse::<i64>().unwrap();

    println!("1:{} 5:{} 15:{} => cpus:{}", load1, load5, load15, nb_cpus);


    // Two cases : cpu_based_load or not. For CPU the real warning is based on warning*nb_cpu
    let mut status = 0;
    let w1 = 1.0;
    let w5 = 1.0;
    let w15 = 1.0;

    let c1 = 2.0;
    let c5 = 2.0;
    let c15 = 2.0;

    // Look if warning < critical
    if c1 < w1 || c5 < w5 || c15 < w15 {
        println!("Error: your critical values should be lower than your warning ones. Please fix it (-w and -c)");
        std::process::exit(2);
    }

    let ratio = nb_cpus as f64;

    // First warning
    if status == 0 && load1 >= w1 * ratio {
        status = 1;
    }
    if status == 0 && load5 >= w5 * ratio {
        status = 1;
    }
    if status == 0 && load15 >= w15 * ratio {
        status = 1;
    }
    // Then critical
    if load1 >= c1 * ratio {
        status = 2;
    }
    if load5 >= c5 * ratio {
        status = 2;
    }
    if load15 >= c15 * ratio {
        status = 2;
    }

    let mut perfdata: String = String::from("");//.to_owned();

    perfdata.push_str(&format!("load1 = {:.2};{:.2};{:.2}; ", load1, w1 * ratio, c1 * ratio));
    perfdata.push_str(&format!("load5 = {:.2};{:.2};{:.2}; ", load5, w5 * ratio, c5 * ratio));
    perfdata.push_str(&format!("load15 = {:.2};{:.2};{:.2}; ", load15, w15 * ratio, c15 * ratio));


    // And    compare    to    limits
    let s_load = format!("{:.2},{:.2},{:.2}", load1, load5, load15);
    if status == 2 {
        println!("Critical: load average is too high {} | {}", s_load, perfdata);
        _do_exit(channel, 2);
        return;
    }

    if status == 1 {
        println!("Warning: load average is very high {} | {}", s_load, perfdata);
        _do_exit(channel, 1);
        return;
    }


    println!("Ok: load average is good {} | {}", s_load, perfdata);


    // Exit
    //let closing = channel.wait_close();
    _do_exit(channel, 0);
    return;
}