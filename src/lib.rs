//! With `libwallet` you can build crypto currency wallets that
//! manage private keys of different kinds saved in a secure storage.
use async_trait::async_trait;
use bip39::Seed;
pub use bip39::{Language, Mnemonic, MnemonicType};

#[cfg(feature = "chain")]
pub mod chain;

#[async_trait]
pub trait Valut {
    async fn store(&self, id: WalletId, secret: &[u8]) -> Result<(), Error>;
    async fn unlock(&self, id: WalletId, password: &str) -> Result<Vec<u8>, Error>;
}

pub type WalletId = Vec<u8>;

/// Wallet is the main interface to manage and interact with accounts.  
pub struct Wallet<'a> {
    id: WalletId,
    mnemonic: Option<Mnemonic>,
    seed: Option<Seed>,
    vault: Option<&'a dyn Valut>,
}

impl<'a> Wallet<'a> {
    /// Generate a new wallet with a 24 word english mnemonic seed
    pub fn new() -> Self {
        let phrase = mnemonic(Language::English);
        Wallet::import(&phrase).unwrap()
    }

    pub fn with_vault(self, vault: &'a dyn Valut) -> Self {
        Wallet {
            vault: Some(vault),
            ..self
        }
    }

    /// Import a wallet from its mnemonic seed
    /// ```
    /// # use libwallet::{Language, Wallet, mnemonic};
    /// let phrase = mnemonic(Language::English);
    /// let mut wallet = Wallet::import(&phrase).unwrap();
    /// ```
    pub fn import(seed_phrase: &str) -> Result<Self, Error> {
        let mnemonic = Mnemonic::from_phrase(seed_phrase, Language::English)
            .map_err(|_| Error::InvalidPhrase)?;
        let seed = Some(Seed::new(&mnemonic, ""));
        Ok(Wallet {
            id: vec![],
            mnemonic: Some(mnemonic),
            seed,
            vault: None,
        })
    }

    pub fn id(&self) -> WalletId {
        self.id.clone()
    }

    /// A locked wallet can use a vault to retrive its secret seed.
    /// ```
    /// # use libwallet::{Wallet, Error, WalletId};
    /// # use libwallet::Valut;
    /// # struct Dummy;
    /// # #[async_trait::async_trait] impl Valut for Dummy {
    /// #   async fn store(&self, _: WalletId, _: &[u8]) -> Result<(), Error> { todo!() }
    /// #   async fn unlock(&self, _: WalletId, _: &str) -> Result<Vec<u8>, Error> { todo!() }
    /// # }
    /// # #[async_std::main] async fn main() -> Result<(), Error> {
    /// # let dummy_vault = Dummy{};
    /// let mut wallet = Wallet::new().with_vault(&dummy_vault);
    /// if wallet.is_locked() {
    ///     wallet.unlock("some password").await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn unlock(&mut self, password: &str) -> Result<(), Error> {
        if !self.is_locked() {
            return Ok(());
        }
        if self.is_locked() && self.vault.is_none() {
            return Err(Error::NoVault);
        }
        let entropy = self.vault.unwrap().unlock(self.id(), password).await?;
        let mnemonic = Mnemonic::from_entropy(&entropy, Language::English)
            .map_err(|_| Error::CorruptedWalletData)?;
        self.seed = Some(Seed::new(&mnemonic, password));
        self.mnemonic = Some(mnemonic);
        Ok(())
    }

    pub fn is_locked(&self) -> bool {
        self.seed.is_none()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid mnemonic phrase")]
    InvalidPhrase,
    #[error("Invalid password")]
    InvalidPasword,
    #[error("Wallet data from the valut is invalid")]
    CorruptedWalletData,
    #[error("Can't unlock, no vault was configured")]
    NoVault,
}

/// Generate a 24 word mnemonic phrase with words in the specified language.
/// ```
/// # use libwallet::{mnemonic, Language};
/// let phrase = mnemonic(Language::English);
/// # let words = phrase.split_whitespace().count();
/// # assert_eq!(words, 24);
/// ```
pub fn mnemonic(lang: Language) -> String {
    Mnemonic::new(MnemonicType::Words24, lang)
        .phrase()
        .to_owned()
}
