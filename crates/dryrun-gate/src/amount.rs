//! Montants en yoctoNEAR, sérialisés en **chaîne décimale** dans le JSON.
//!
//! Un yoctoNEAR dépasse largement la portée d'un `f64` JavaScript : passer par
//! une chaîne évite toute perte de précision dans la page web et dans `serde_json`.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use dryrun_core::types::Yocto;

/// Montant en yoctoNEAR.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Amount(pub Yocto);

impl Amount {
    /// Valeur brute en yoctoNEAR.
    pub fn yocto(self) -> Yocto {
        self.0
    }

    /// Représentation NEAR lisible (six décimales significatives).
    pub fn near(self) -> String {
        dryrun_core::card::format_ynear(self.0 as i128)
    }
}

impl From<Yocto> for Amount {
    fn from(value: Yocto) -> Self {
        Amount(value)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for Amount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

/// Accepte aussi bien `"1000000000000000000000000"` que `1000000` (petits montants).
struct AmountVisitor;

impl Visitor<'_> for AmountVisitor {
    type Value = Amount;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("un montant yoctoNEAR, en chaîne décimale ou en entier")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Amount, E> {
        let cleaned = value.trim().replace('_', "");
        cleaned
            .parse::<Yocto>()
            .map(Amount)
            .map_err(|_| E::custom(format!("montant yoctoNEAR invalide : {value}")))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Amount, E> {
        Ok(Amount(value as Yocto))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Amount, E> {
        if value < 0 {
            return Err(E::custom("un montant ne peut pas être négatif"));
        }
        Ok(Amount(value as Yocto))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Amount, E> {
        if value < 0.0 || !value.is_finite() {
            return Err(E::custom("montant invalide"));
        }
        Ok(Amount(value as Yocto))
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(AmountVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_en_chaine() {
        let amount = Amount(10u128.pow(24));
        let json = serde_json::to_string(&amount).expect("sérialisation");
        assert_eq!(json, "\"1000000000000000000000000\"");
        let back: Amount = serde_json::from_str(&json).expect("désérialisation");
        assert_eq!(back, amount);
    }

    #[test]
    fn accepte_un_entier_json() {
        let back: Amount = serde_json::from_str("1500000").expect("désérialisation");
        assert_eq!(back, Amount(1_500_000));
    }

    #[test]
    fn refuse_un_montant_textuel_invalide() {
        assert!(serde_json::from_str::<Amount>("\"pas un nombre\"").is_err());
    }
}
