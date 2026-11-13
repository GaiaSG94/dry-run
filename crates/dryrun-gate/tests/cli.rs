//! Tests de l'analyse des arguments de la ligne de commande.

use dryrun_core::types::Demande;
use dryrun_gate::cli::{parse, parse_near, Command, Network};
use dryrun_gate::lang::Lang;

/// Construit un vecteur d'arguments à partir de mots.
fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| w.to_string()).collect()
}

#[test]
fn parse_near_accepte_les_decimaux() {
    assert_eq!(parse_near("1").expect("1 NEAR"), 10u128.pow(24));
    assert_eq!(parse_near("0.5").expect("0.5 NEAR"), 5 * 10u128.pow(23));
    assert_eq!(parse_near("2.").expect("2 NEAR"), 2 * 10u128.pow(24));
    assert_eq!(parse_near("0").expect("0 NEAR"), 0);
}

#[test]
fn parse_near_refuse_les_textes() {
    assert!(parse_near("beaucoup").is_err());
    assert!(parse_near("1.2.3").is_err());
}

#[test]
fn propose_transfert_exige_une_intention() {
    let args = argv(&["propose", "--to", "bob.test.near", "--amount", "1"]);
    assert!(parse(&args).is_err());
}

#[test]
fn propose_transfert_construit_la_demande() {
    let args = argv(&[
        "propose",
        "--to",
        "bob.test.near",
        "--amount",
        "1",
        "--intent",
        "payer 1 NEAR à bob",
        "--beneficiary",
        "bob.test.near",
    ]);
    match parse(&args).expect("commande valide") {
        Command::Propose { demand, intent } => {
            assert_eq!(
                demand,
                Demande::Transfer {
                    to: "bob.test.near".to_string(),
                    amount: 10u128.pow(24),
                }
            );
            assert_eq!(intent.summary, "payer 1 NEAR à bob");
            assert_eq!(intent.beneficiaries, vec!["bob.test.near".to_string()]);
        }
        other => panic!("commande inattendue : {other:?}"),
    }
}

#[test]
fn propose_appel_construit_la_demande() {
    let args = argv(&[
        "propose",
        "--call",
        "good-dex.test.near",
        "--method",
        "swap",
        "--args",
        "{\"min_out\":1}",
        "--deposit",
        "2",
        "--intent",
        "swapper 2 NEAR",
        "--min-in",
        "TEST:1.9",
    ]);
    match parse(&args).expect("commande valide") {
        Command::Propose { demand, intent } => {
            match demand {
                Demande::FunctionCall {
                    contract,
                    method,
                    deposit,
                    ..
                } => {
                    assert_eq!(contract, "good-dex.test.near");
                    assert_eq!(method, "swap");
                    assert_eq!(deposit, 2 * 10u128.pow(24));
                }
                other => panic!("demande inattendue : {other:?}"),
            }
            assert_eq!(intent.min_in.len(), 1);
            assert_eq!(intent.min_in[0].0, "TEST");
            // TEST a 18 décimales : 1.9 TEST = 19 * 10^17
            assert_eq!(intent.min_in[0].1, 19 * 10u128.pow(17));
        }
        other => panic!("commande inattendue : {other:?}"),
    }
}

#[test]
fn serve_lit_le_port() {
    let args = argv(&["serve", "--port", "9090", "--web-dir", "web"]);
    match parse(&args).expect("commande valide") {
        Command::Serve {
            port,
            web_dir,
            network,
        } => {
            assert_eq!(port, 9090);
            assert_eq!(web_dir, "web");
            assert_eq!(network, Network::local());
        }
        other => panic!("commande inattendue : {other:?}"),
    }
}

#[test]
fn serve_testnet_exige_un_coffre() {
    let args = argv(&["serve", "--network", "testnet"]);
    let err = parse(&args).expect_err("testnet sans coffre doit échouer");
    assert!(err.to_string().contains("--vault"), "erreur : {err}");
}

#[test]
fn serve_testnet_avec_coffre_est_reconnu() {
    let args = argv(&[
        "serve",
        "--network",
        "testnet",
        "--vault",
        "dryrun-vault-1112.testnet",
    ]);
    match parse(&args).expect("commande valide") {
        Command::Serve { network, .. } => match network {
            Network::Chain { vault, name, .. } => {
                assert_eq!(vault, "dryrun-vault-1112.testnet");
                assert_eq!(name, "testnet");
            }
            other => panic!("réseau inattendu : {other:?}"),
        },
        other => panic!("commande inattendue : {other:?}"),
    }
}

#[test]
fn mcp_accepte_la_langue_et_le_reseau() {
    let args = argv(&[
        "mcp",
        "--network",
        "testnet",
        "--vault",
        "dryrun-vault-1112.testnet",
        "--lang",
        "fr",
    ]);
    match parse(&args).expect("commande valide") {
        Command::Mcp { network, lang } => {
            assert_eq!(lang, Lang::Fr);
            assert!(matches!(network, Network::Chain { .. }));
        }
        other => panic!("commande inattendue : {other:?}"),
    }
}

#[test]
fn mcp_rejette_une_langue_inconnue() {
    let args = argv(&["mcp", "--lang", "italien"]);
    assert!(parse(&args).is_err());
}

#[test]
fn mcp_par_defaut_est_en_anglais_et_en_local() {
    match parse(&argv(&["mcp"])).expect("commande valide") {
        Command::Mcp { network, lang } => {
            assert_eq!(lang, Lang::En);
            assert_eq!(network, Network::local());
        }
        other => panic!("commande inattendue : {other:?}"),
    }
}

#[test]
fn commandes_simples_sont_reconnues() {
    assert_eq!(parse(&argv(&["freeze"])).expect("freeze"), Command::Freeze);
    assert_eq!(parse(&argv(&["status"])).expect("status"), Command::Status);
    assert_eq!(parse(&argv(&["log"])).expect("log"), Command::Log);
    assert_eq!(parse(&argv(&[])).expect("aide"), Command::Help);
    assert!(parse(&argv(&["inconnue"])).is_err());
}
