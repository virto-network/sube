#![feature(trait_alias)]
// #![feature(result_option_inspect)]
#![cfg_attr(not(feature = "std"), no_std)]
//! `libwallet` is the one-stop tool to build easy, slightly opinionated crypto wallets
//! that run in all kinds of environments and plattforms including embedded hardware,
//! mobile apps or the Web.
//! It's easy to extend implementing different vault backends and it's designed to
//! be compatible with all kinds of key formats found in many different blockchains.
#[cfg(not(any(feature = "sr25519")))]
compile_error!("Enable at least one type of signature algorithm");

mod account;
mod key_pair;
#[cfg(feature = "substrate")]
mod substrate_ext;

use arrayvec::ArrayVec;
use core::{convert::TryInto, fmt};
use key_pair::any::AnySignature;

#[cfg(feature = "mnemonic")]
use mnemonic;

pub use account::Account;
pub use key_pair::*;
#[cfg(feature = "mnemonic")]
pub use mnemonic::{Language, Mnemonic};
pub use vault::{RootAccount, Vault};
pub mod vault;

const MSG_MAX_SIZE: usize = u8::MAX as usize;
pub type Message = ArrayVec<u8, { MSG_MAX_SIZE }>;

/// Wallet is the main interface to interact with the accounts of a user.
///
/// Before being able to sign messages a wallet must be unlocked using valid credentials
/// supported by the underlying vault.
///
/// Wallets can hold many user defined accounts and always have one account set as "default",
/// if no account is set as default one is generated and will be used to sign messages when no account is specified.
///
/// Wallets also support queuing and bulk signing of messages in case transactions need to be reviewed before signing.
#[derive(Debug)]
pub struct Wallet<V: Vault, const A: usize = 5, const M: usize = A> {
    vault: V,
    is_locked: bool,
    default_account: Account,
    accounts: ArrayVec<Account, A>,
    pending_sign: ArrayVec<(Message, Option<u8>), M>, // message -> account index or default
}

impl<V, const A: usize, const M: usize> Wallet<V, A, M>
where
    V: Vault,
{
    /// Create a new Wallet with a default account
    pub fn new(vault: V) -> Self {
        Wallet {
            vault,
            default_account: Account::new(None),
            accounts: ArrayVec::new_const(),
            pending_sign: ArrayVec::new(),
            is_locked: true,
        }
    }

    /// Get the account currently set as default
    pub fn default_account(&self) -> &Account {
        &self.default_account
    }

    /// Use credentials to unlock the vault.
    ///
    /// ```
    /// # use libwallet::{Wallet, Error, vault, Vault};
    /// # use std::convert::TryInto;
    /// # type Result = std::result::Result<(), Error<<vault::Simple as Vault>::Error>>;
    /// # #[async_std::main] async fn main() -> Result {
    /// # let vault = vault::Simple::generate(&mut rand_core::OsRng);
    /// let mut wallet: Wallet<_> = Wallet::new(vault);
    /// if wallet.is_locked() {
    ///     wallet.unlock(None).await?;
    /// }
    ///
    /// assert!(!wallet.is_locked());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unlock(&mut self, cred: impl Into<V::Credentials>) -> Result<(), Error<V::Error>> {
        if self.is_locked() {
            let vault = &mut self.vault;
            let def = &mut self.default_account;

            vault
                .unlock(cred, |root| {
                    def.unlock(root);
                })
                .await
                .map_err(|e| Error::Vault(e))?;
            self.is_locked = false;
        }
        Ok(())
    }

    /// Check if the vault has been unlocked.
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }

    /// Sign a message with the default account and return the signature.
    /// The wallet needs to be unlocked.
    ///
    /// ```
    /// # use libwallet::{Wallet, vault, Error, Signer, Vault};
    /// # type Result = std::result::Result<(), Error<<vault::Simple as Vault>::Error>>;
    /// # #[async_std::main] async fn main() -> Result {
    /// # let vault = vault::Simple::generate(&mut rand_core::OsRng);
    /// let mut wallet: Wallet<_> = Wallet::new(vault);
    /// wallet.unlock(None).await?;
    ///
    /// let msg = &[0x12, 0x34, 0x56];
    /// let signature = wallet.sign(msg);
    ///
    /// assert!(wallet.default_account().verify(msg, signature.as_ref()));
    /// # Ok(()) }
    /// ```
    pub fn sign(&self, message: &[u8]) -> impl Signature {
        assert!(!self.is_locked());
        self.default_account().sign_msg(message)
    }

    /// Save data to be signed some time later.
    ///
    /// ```
    /// # use libwallet::{Wallet, vault, Error, Vault};
    /// # type Result = std::result::Result<(), Error<<vault::Simple as Vault>::Error>>;
    /// # #[async_std::main] async fn main() -> Result {
    /// # let vault = vault::Simple::generate(&mut rand_core::OsRng);
    /// let mut wallet: Wallet<_> = Wallet::new(vault);
    /// wallet.sign_later(&[0x01, 0x02, 0x03]);
    ///
    /// assert_eq!(wallet.pending().count(), 1);
    /// # Ok(()) }
    /// ```
    pub fn sign_later<T>(&mut self, message: T)
    where
        T: AsRef<[u8]>,
    {
        let msg = message.as_ref();
        let msg = msg
            .try_into()
            .unwrap_or_else(|_| msg[..MSG_MAX_SIZE].try_into().unwrap());
        self.pending_sign.push((msg, None));
    }

    /// Try to sign all messages in the queue returning the list of signatures
    ///
    /// ```
    /// # use libwallet::{Wallet, vault, Error, Vault};
    /// # type Result = std::result::Result<(), Error<<vault::Simple as Vault>::Error>>;
    /// # #[async_std::main] async fn main() -> Result {
    /// # let vault = vault::Simple::generate(&mut rand_core::OsRng);
    /// let mut wallet: Wallet<_> = Wallet::new(vault);
    /// wallet.unlock(None).await?;
    ///
    /// wallet.sign_later(&[0x01, 0x02, 0x03]);
    /// wallet.sign_later(&[0x04, 0x05, 0x06]);
    /// let signatures = wallet.sign_pending();
    ///
    /// assert_eq!(signatures.len(), 2);
    /// assert_eq!(wallet.pending().count(), 0);
    /// # Ok(()) }
    /// ```
    pub fn sign_pending(&mut self) -> ArrayVec<AnySignature, M> {
        let mut signatures = ArrayVec::new();
        for (msg, a) in self.pending_sign.take() {
            let account = a
                .map(|idx| self.account(idx))
                .unwrap_or_else(|| self.default_account());
            signatures.push(account.sign_msg(&msg));
        }
        signatures
    }

    /// Iteratate over the messages pending for signature for all the accounts.
    ///
    /// ```
    /// # use libwallet::{Wallet, vault, Error, Vault};
    /// # type Result = std::result::Result<(), Error<<vault::Simple as Vault>::Error>>;
    /// # #[async_std::main] async fn main() -> Result {
    /// # let vault = vault::Simple::generate(&mut rand_core::OsRng);
    /// let mut wallet: Wallet<_> = Wallet::new(vault);
    /// wallet.sign_later(&[0x01]);
    /// wallet.sign_later(&[0x02]);
    /// wallet.sign_later(&[0x03]);
    ///
    /// assert_eq!(wallet.pending().count(), 3);
    /// # Ok(()) }
    /// ```
    pub fn pending(&self) -> impl Iterator<Item = &[u8]> {
        self.pending_sign.iter().map(|(msg, _)| msg.as_ref())
    }

    #[cfg(feature = "subaccounts")]
    pub fn new_account(&mut self, name: &str, derivation_path: &str) -> Result<&Account<V::Pair>> {
        let root = self.root_account()?;
        let subaccount = root
            .derive_subaccount(name, derivation_path)
            .ok_or(Error::DeriveError)?;
        self.subaccounts.insert(name.to_string(), subaccount);
        Ok(self.subaccounts.get(name).unwrap())
    }

    fn account(&self, idx: u8) -> &Account {
        &self.accounts[idx as usize]
    }
}

/// Represents the blockchain network in use by an account
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Network {
    // For substrate based blockchains commonly formatted as SS58
    // that are distinguished by their address prefix. 42 is the generic prefix.
    #[cfg(feature = "substrate")]
    Substrate(u16),
    // Space for future supported networks(e.g. ethereum, bitcoin)
    _Missing,
}

impl Default for Network {
    fn default() -> Self {
        #[cfg(feature = "substrate")]
        let net = Network::Substrate(42);
        #[cfg(not(feature = "substrate"))]
        let net = Network::_Missing;
        net
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            #[cfg(feature = "substrate")]
            Self::Substrate(_) => write!(f, "substrate"),
            _ => write!(f, ""),
        }
    }
}

#[derive(Debug)]
pub enum Error<V> {
    Vault(V),
    Locked,
    DeriveError,
    #[cfg(feature = "mnemonic")]
    InvalidPhrase,
}

impl<V> fmt::Display for Error<V>
where
    V: fmt::Debug + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Vault(e) => write!(f, "Vault error: {}", e),
            Error::Locked => write!(f, "Locked"),
            Error::DeriveError => write!(f, "Cannot derive"),
            #[cfg(feature = "mnemonic")]
            Error::InvalidPhrase => write!(f, "Invalid phrase"),
        }
    }
}

#[cfg(feature = "std")]
impl<V> std::error::Error for Error<V> where V: fmt::Debug + fmt::Display {}

#[cfg(feature = "mnemonic")]
impl<V> From<mnemonic::Error> for Error<V> {
    fn from(_: mnemonic::Error) -> Self {
        Error::InvalidPhrase
    }
}

mod util {
    use core::{iter, ops};

    #[cfg(feature = "rand")]
    pub fn random_bytes<R, const S: usize>(rng: &mut R) -> [u8; S]
    where
        R: rand_core::CryptoRng + rand_core::RngCore,
    {
        let mut bytes = [0u8; S];
        rng.fill_bytes(&mut bytes);
        bytes
    }

    #[cfg(feature = "rand")]
    pub fn gen_phrase<R>(rng: &mut R, lang: mnemonic::Language) -> mnemonic::Mnemonic
    where
        R: rand_core::CryptoRng + rand_core::RngCore,
    {
        let seed = random_bytes::<_, 32>(rng);
        let phrase = mnemonic::Mnemonic::from_entropy_in(lang, seed.as_ref()).expect("seed valid");
        phrase
    }

    macro_rules! seed_from_entropy {
        ($seed: ident, $pin: expr) => {
            #[cfg(feature = "util_pin")]
            let protected_seed = $pin.protect::<64>($seed);
            #[cfg(feature = "util_pin")]
            let $seed = &protected_seed;
            #[cfg(not(feature = "util_pin"))]
            let _ = &$pin; // use the variable to avoid warning
        };
    }

    pub(crate) use seed_from_entropy;

    /// A simple pin credential that can be used to add some
    /// extra level of protection to seeds stored in vaults
    #[derive(Default, Copy, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Pin(u16);

    impl Pin {
        const LEN: usize = 4;

        #[cfg(feature = "util_pin")]
        pub fn protect<const S: usize>(&self, data: &[u8]) -> [u8; S] {
            use hmac::Hmac;
            use pbkdf2::pbkdf2;
            use sha2::Sha512;

            let salt = {
                let mut s = [0; 10];
                s.copy_from_slice(b"mnemonic\0\0");
                let [b1, b2] = self.to_le_bytes();
                s[8] = b1;
                s[9] = b2;
                s
            };
            let mut seed = [0; S];
            // using same hashing strategy as Substrate to have some compatibility
            // when pin is 0(no pin) we produce the same addresses
            let len = self.eq(&0).then_some(salt.len() - 2).unwrap_or(salt.len());
            pbkdf2::<Hmac<Sha512>>(data, &salt[..len], 2048, &mut seed);
            seed
        }
    }

    // Use 4 chars long hex string as pin. i.e. "ABCD", "1234"
    impl<'a> From<&'a str> for Pin {
        fn from(s: &str) -> Self {
            let l = s.len().min(Pin::LEN);
            let chars = s
                .chars()
                .take(l)
                .chain(iter::repeat('0').take(Pin::LEN - l));
            Pin(chars
                .map(|c| c.to_digit(16).unwrap_or(0))
                .enumerate()
                .fold(0, |pin, (i, d)| {
                    pin | ((d as u16) << (Pin::LEN - 1 - i) * Pin::LEN)
                }))
        }
    }

    impl<'a> From<Option<&'a str>> for Pin {
        fn from(p: Option<&'a str>) -> Self {
            p.unwrap_or("").into()
        }
    }

    impl From<()> for Pin {
        fn from(_: ()) -> Self {
            Self(0)
        }
    }

    impl From<u16> for Pin {
        fn from(n: u16) -> Self {
            Self(n)
        }
    }

    impl ops::Deref for Pin {
        type Target = u16;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    #[test]
    fn pin_parsing() {
        for (s, expected) in [
            ("0000", 0),
            // we only take the first 4 characters and ignore the rest
            ("0000001", 0),
            // non hex chars are ignored and defaulted to 0, here a,d are kept
            ("zasdasjgkadg", 0x0A0D),
            ("ABCD", 0xABCD),
            ("1000", 0x1000),
            ("000F", 0x000F),
            ("FFFF", 0xFFFF),
        ] {
            let pin = Pin::from(s);
            assert_eq!(
                *pin, expected,
                "(input:\"{}\", l:{:X} == r:{:X})",
                s, *pin, expected
            );
        }
    }
}
