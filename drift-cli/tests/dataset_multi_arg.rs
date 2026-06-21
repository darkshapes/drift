use clap::Parser;
use drift_cli::{Cli, Commands};

#[test]
fn test_cli_dataset_single_arg() {
    let args = vec!["drift", "train", "--dataset-urls", "https://huggingface.co/datasets/user/data"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Train { dataset_urls, .. } => {
            assert_eq!(dataset_urls.len(), 1);
            assert_eq!(dataset_urls[0], "https://huggingface.co/datasets/user/data");
        }
        _ => panic!("expected Train command"),
    }
}

#[test]
fn test_cli_dataset_multiple_args() {
    let args = vec![
        "drift",
        "train",
        "--dataset-urls",
        "https://huggingface.co/datasets/user/data1",
        "--dataset-urls",
        "https://huggingface.co/datasets/user/data2",
        "--dataset-urls",
        "/local/datasets/data3",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Train { dataset_urls, .. } => {
            assert_eq!(dataset_urls.len(), 3);
            assert_eq!(dataset_urls[0], "https://huggingface.co/datasets/user/data1");
            assert_eq!(dataset_urls[1], "https://huggingface.co/datasets/user/data2");
            assert_eq!(dataset_urls[2], "/local/datasets/data3");
        }
        _ => panic!("expected Train command"),
    }
}

#[test]
fn test_cli_dataset_empty() {
    let args = vec!["drift", "train"];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Train { dataset_urls, .. } => {
            assert!(dataset_urls.is_empty());
        }
        _ => panic!("expected Train command"),
    }
}

#[test]
fn test_cli_dataset_with_other_args() {
    let args = vec![
        "drift",
        "train",
        "--train-repo-url",
        "https://github.com/user/repo",
        "--dataset-urls",
        "https://huggingface.co/datasets/user/data1",
        "--dataset-urls",
        "https://huggingface.co/datasets/user/data2",
    ];
    let cli = Cli::try_parse_from(args).unwrap();
    match cli.command {
        Commands::Train {
            train_repo_url,
            dataset_urls,
            ..
        } => {
            assert_eq!(dataset_urls.len(), 2);
            assert!(train_repo_url.is_some());
        }
        _ => panic!("expected Train command"),
    }
}

#[test]
fn test_cli_dataset_order_independence() {
    let args1 = vec![
        "drift",
        "train",
        "--dataset-urls",
        "https://example.com/a",
        "--dataset-urls",
        "https://example.com/b",
    ];
    let args2 = vec![
        "drift",
        "train",
        "--dataset-urls",
        "https://example.com/b",
        "--dataset-urls",
        "https://example.com/a",
    ];

    let cli1 = Cli::try_parse_from(args1).unwrap();
    let cli2 = Cli::try_parse_from(args2).unwrap();

    match (cli1.command, cli2.command) {
        (Commands::Train { dataset_urls: d1, .. }, Commands::Train { dataset_urls: d2, .. }) => {
            assert_eq!(d1.len(), 2);
            assert_eq!(d2.len(), 2);
        }
        _ => panic!("expected Train commands"),
    }
}
