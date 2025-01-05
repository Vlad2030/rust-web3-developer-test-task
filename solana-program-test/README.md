# How to Test

## Create `.env` file

example:

```bash
CLUSTER=devnet
PROGRAM_ID=9LUVrpy2nHxk57DVUkKfFZTbL9tXGKrKgmgVJGf1LK33
KEYPAIR=your_keypair
```

## Test

```bash
RUST_TEST_TASKS=1 cargo test
```

> run twice for withdraw test
