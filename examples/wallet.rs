use clap::{App, Arg};
use libwallet::{self, sr25519::Pair, Mnemonic, Result, SimpleVault};

type Wallet = libwallet::Wallet<SimpleVault<Pair>>;

#[async_std::main]
async fn main() -> Result<()> {
    let matches = App::new("Wallet Generator")
        .version("0.1.0")
        .author("Virto Team <we@virto.team>")
        .about("Generates Wallet Account")
        .arg(
            Arg::with_name("seed")
                .short("s")
                .long("from-seed")
                .value_name("MNEMONIC")
                .help("Generates a wallet address from mnemonic."),
        )
        .arg(
            Arg::with_name("network")
                .short("n")
                .long("network")
                .value_name("NETWORK")
                .help("Formats the address to specified network."),
        )
        .get_matches();
    let network: &str = matches.value_of("network").unwrap_or("substrate");

    let phrase = match matches.value_of("seed") {
        Some(mnemonic) => mnemonic.parse()?,
        None => {
            let entropy = [0; 32];
            Mnemonic::from_entropy(entropy)?
        }
    };
    println!("Secret Key: \"{}\"", &phrase);

    let vault = SimpleVault::<Pair>::from_phrase(phrase);
    let mut wallet = Wallet::new(vault).unlock("").await?;
    let account = wallet.switch_default_network(network)?;

    println!("Public key ({}): {:?}", account.network(), account.public());
    Ok(())
}
