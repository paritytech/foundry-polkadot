use alloy_sol_types::sol;

sol!(
    #[derive(Debug)]
    SimpleStorage,
    "test-data/SimpleStorage.json"
);

sol!(
    #[derive(Debug)]
    DoubleStorage,
    "test-data/DoubleStorage.json"
);

sol!(
    #[derive(Debug)]
    Multicall,
    "test-data/Multicall.json"
);

sol!(
    #[derive(Debug)]
    SimpleStorageCaller,
    "test-data/SimpleStorageCaller.json"
);

sol!(
    #[derive(Debug)]
    ImmutableStorage,
    "test-data/ImmutableStorage.json"
);
