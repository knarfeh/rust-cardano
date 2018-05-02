use wallet_crypto::{wallet, hdwallet, bip44, bip39};
use wallet_crypto::util::base58;
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use config::{Config};
use account::{Account};
use rand;

use termion::{style, color};
use termion::input::TermRead;
use std::io::{Write, stdout, stdin};

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet(wallet::Wallet);
impl Wallet {
    fn generate(seed: bip39::Seed) -> Self {
        Wallet(wallet::Wallet::new_from_bip39(&seed))
    }
}

impl HasCommand for Wallet {
    type Output = Option<Config>;

    fn clap_options<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name("wallet")
            .about("wallet management")
            .subcommand(SubCommand::with_name("generate")
                .about("generate a new wallet")
                .arg(Arg::with_name("LANGUAGE")
                    .long("language")
                    .takes_value(true)
                    .value_name("LANGUAGE")
                    .possible_values(&["english"])
                    .help("use the given language for the mnemonic")
                    .required(false)
                    .default_value(r"english")
                )
                .arg(Arg::with_name("MNEMONIC SIZE")
                    .long("mnemonic-size")
                    .takes_value(true)
                    .value_name("MNEMOENIC_SIZE")
                    .possible_values(&["12-words", "15-words", "18-words", "21-words", "24-words"])
                    .help("set the size of the mnemonic words")
                    .required(false)
                    .default_value(r"15-words")
                )
                .arg(Arg::with_name("PASSWORD")
                    .long("--password")
                    .takes_value(true)
                    .value_name("PASSWORD")
                    .help("set the password from the CLI instead of prompting for it. It is quite unsafe as the password can be visible from your shell history.")
                    .required(false)
                )
            )
            .subcommand(SubCommand::with_name("recover")
                .about("recover a wallet from bip39 mnemonics")
                .arg(Arg::with_name("LANGUAGE")
                    .long("language")
                    .takes_value(true)
                    .value_name("LANGUAGE")
                    .possible_values(&["english"])
                    .help("use the given language for the mnemonic")
                    .required(false)
                    .default_value(r"english")
                )
                .arg(Arg::with_name("PASSWORD")
                    .long("--password")
                    .takes_value(true)
                    .value_name("PASSWORD")
                    .help("set the password from the CLI instead of prompting for it. It is quite unsafe as the password can be visible from your shell history.")
                    .required(false)
                )
            )
            .subcommand(SubCommand::with_name("address")
                .about("create an address with the given options")
                .arg(Arg::with_name("is_internal").long("internal").help("to generate an internal address (see BIP44)"))
                .arg(Arg::with_name("account").help("account to generate an address in").index(1).required(true))
                .arg(Arg::with_name("indices")
                    .help("list of indices for the addresses to create")
                    .multiple(true)
                )
            )
    }
    fn run(config: Config, args: &ArgMatches) -> Self::Output {
        let mut cfg = config;
        match args.subcommand() {
            ("generate", Some(opts)) => {
                // expect no existing wallet
                assert!(cfg.wallet.is_none());
                let language    = value_t!(opts.value_of("LANGUAGE"), String).unwrap(); // we have a default value
                let mnemonic_sz = value_t!(opts.value_of("MNEMONIC SIZE"), bip39::Type).unwrap();
                let password    = value_t!(opts.value_of("PASSWORD"), String).ok();
                let seed = generate_entropy(language, password, mnemonic_sz);
                cfg.wallet = Some(Wallet::generate(seed));
                let _storage = cfg.get_storage().unwrap();
                Some(cfg) // we need to update the config's wallet
            },
            ("address", Some(opts)) => {
                // expect existing wallet
                assert!(cfg.wallet.is_some());
                match &cfg.wallet {
                    &None => panic!("No wallet created, see `wallet generate` command"),
                    &Some(ref wallet) => {
                        let addr_type = if opts.is_present("is_internal") {
                            bip44::AddrType::Internal
                        } else {
                            bip44::AddrType::External
                        };
                        let account_name = opts.value_of("account")
                            .and_then(|s| Some(Account::new(s.to_string())))
                            .unwrap();
                        let account = match cfg.find_account(&account_name) {
                            None => panic!("no account {:?}", account_name),
                            Some(r) => r,
                        };
                        let indices = values_t!(opts.values_of("indices"), u32).unwrap_or_else(|_| vec![0]);

                        let addresses = wallet.0.gen_addresses(account, addr_type, indices);
                        for addr in addresses {
                            println!("{}", base58::encode(&addr.to_bytes()));
                        };
                        None // we don't need to update the wallet
                    }
                }
            },
            _ => {
                println!("{}", args.usage());
                ::std::process::exit(1);
            },
        }
    }
}

fn generate_entropy(language: String, opt_pwd: Option<String>, t: bip39::Type) -> bip39::Seed {
    assert!(language == "english");
    let dic = &bip39::dictionary::ENGLISH;

    let pwd = match opt_pwd {
        Some(pwd) => pwd,
        None => {
            let stdout = stdout();
            let mut stdout = stdout.lock();
            let stdin = stdin();
            let mut stdin = stdin.lock();

            stdout.write_all(b"password: ").unwrap();
            stdout.flush().unwrap();

            let pwd = stdin.read_passwd(&mut stdout).unwrap().unwrap_or("".to_string());
            stdout.write_all(b"\n").unwrap();
            stdout.flush().unwrap();
            pwd
        }
    };

    let entropy = bip39::Entropy::generate(t, rand::random);

    let mnemonic = entropy.to_mnemonics().to_string(dic);
    println!("mnemonic: {}", mnemonic);

    bip39::Seed::from_mnemonic_string(&mnemonic, pwd.as_bytes())
}