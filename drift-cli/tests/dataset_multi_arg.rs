use clap::Parser;
use drift_cli::{Cli, Commands};

#[test]
fn test_cli_dataset_single_arg() {
    let args = vec!["drift", "train", "--dataset", "https://huggingface.co/datasets/user/data"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Train { dataset, .. } => {
            assert_eq!(dataset.len(), 1);
            assert_eq!(dataset[0], "https://huggingface.co/datasets/user/data");
        }
        _ => panic!("expected Train command"),
    }
}

#[test]
fn test_cli_dataset_multiple_args() {
    let args = vec![
        "drift",
        "train",
        "--dataset",
        "https://huggingface.co/datasets/user/data1",
        "--dataset",
        "https://huggingface.co/datasets/user/data2",
        "--dataset",
        "/local/datasets/data3",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Train { dataset, .. } => {
            assert_eq!(dataset.len(), 3);
            assert_eq!(dataset[0], "https://huggingface.co/datasets/user/data1");
            assert_eq!(dataset[1], "https://huggingface.co/datasets/user/data2");
            assert_eq!(dataset[2], "/local/datasets/data3");
        }
        _ => panic!("expected Train command"),
    }
}

#[test]
fn test_cli_dataset_empty() {
    let args = vec!["drift", "train"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Train { dataset, .. } => {
            assert!(dataset.is_empty());
        }
        _ => panic!("expected Train command"),
    }
}

#[test]
fn test_cli_dataset_with_other_args() {
    let args = vec![
        "drift",
        "train",
        "--repo",
        "https://github.com/user/repo",
        "--dataset",
        "https://huggingface.co/datasets/user/data1",
        "--dataset",
        "https://huggingface.co/datasets/user/data2",
        "--epochs",
        "20",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Train {
            repo,
            dataset,
            epochs,
            ..
        } => {
            assert_eq!(dataset.len(), 2);
            assert!(repo.is_some());
            assert_eq!(epochs, 20u32);
        }
        _ => panic!("expected Train command"),
    }
}

#[test]
fn test_cli_dataset_order_independence() {
    let args1 = vec![
        "drift",
        "train",
        "--dataset",
        "https://example.com/a",
        "--dataset",
        "https://example.com/b",
    ];
    let args2 = vec![
        "drift",
        "train",
        "--dataset",
        "https://example.com/b",
        "--dataset",
        "https://example.com/a",
    ];

    let cli1 = Cli::try_parse_from(args1).unwrap();
    let cli2 = Cli::try_parse_from(args2).unwrap();

    match (cli1.command, cli2.command) {
        (Commands::Train { dataset: d1, .. }, Commands::Train { dataset: d2, .. }) => {
            assert_eq!(d1.len(), 2);
            assert_eq!(d2.len(), 2);
        }
        _ => panic!("expected Train commands"),
    }
}