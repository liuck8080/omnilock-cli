# Open Transaction tutorial

## Deploy the omnilock with open transaction feature enabled.
If you use testnet or mainnet, you should use the already deployed script, so skip this step.
### Requirements
- ckb-cli at least 1.3.0, this version enabled a deploy command.
- open transaction feature enabled omnilock. You can build it from [ckb-production-script](https://github.com/nervosnetwork/ckb-production-scripts/tree/opentx)
### Deploy steps
1. Prepare a deploy working directory like `deploy-files`, and make 2 directories named `contracts` and `migrations`
2. Copy the `omni_lock` from `ckb-product-script` output directory into `contracts`.
3. Create a file named `deployment.toml` with content like this:
```toml
[[cells]]
name = "omni_lock"
enable_type_id = true
location = { file = "contracts/omni_lock" }

# reference to on-chain cells, this config is referenced by dep_groups.cells
[[cells]]
name = "secp256k1_data"
enable_type_id = false
location = { tx_hash = "<genesis-cellbase-tx-hash>", index = 3 }

# Dep group cells
[[dep_groups]]
name = "omni_lock_dep"
cells = [
  "secp256k1_data",
  "omni_lock"
]

# Replace with your own lock if you want to unlock deployed cells.
# For example the secp256k1 lock
[lock]
code_hash = "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8"
args = "<sighash-address-lock-args>"
hash_type = "type"
```
The `genesis-cellbase-tx-hash` can get from `ckb-cli util genesis-scripts` in the output's `secp256k1_data` field.

4. Run the following command one
```sh
ckb-cli deploy gen-txs \
    --deployment-config ./deployment.toml \
    --migration-dir ./migrations \
    --from-address <your-account> \
    --info-file ./info.json \
    --sign-now
ckb-cli deploy apply-txs --migration-dir ./migrations --info-file ./info.json
```
The deployment information is in the info.json, you can also get 2 transaction hashes from the output. The 2 transactions will be in a same block.

## Configuration for opentx
Create a file ~/.omnilock.yaml with content:
```yaml
# The tansaction where the omnilock script was deployed, in hex mode;
# omnilock_tx_hash can compose the output content.
omnilock_tx_hash: "0000000000000000000000000000000000000000000000000000000000000000"
# The index where the omilock script was deployed.
omnilock_index: 0
# The ckb_rpc url
ckb_rpc: "http://127.0.0.1:8114"
```
Replace the omnilock_tx_hash with the correct transaction hash of last step. You can find it in `info.json`, under section `new_recipe`.`cell_recipes`, or the output of command `ckb-cli deploy apply-txs --migration-dir ./migrations --info-file ./info.json`, next to `cell_tx`.

## Simple transaction
### Pubkey hash open transaction.
1. Build a sighash opentx address.
. Generate the transaction
```sh
omnilock-cli build-address pubkey-hash --sighash-address ckt1qyqt8xpk328d89zgl928nsgh3lelch33vvvq5u3024 --flags opentx
```
Here is the example output, the result varys when `--sighash-address` and `omnilock_tx_hash` varys.
```json
{
  "lock-arg": "0x00b398368a8ed39448f95479c1178ff3fc5e31631810",
  "lock-hash": "0xe518591bac39ba32ca7ae6d2e1ffb5998a84f6d1c635767e41edc9e60d247671",
  "mainnet": "ckb1qqhj387qfxcxsa7xthycgf8q5k5yl8n2rfeacyn258euza9shmfsgqgqkwvrdz5w6w2y372508q30rlnl30rzccczqfe6vpk",
  "testnet": "ckt1qqhj387qfxcxsa7xthycgf8q5k5yl8n2rfeacyn258euza9shmfsgqgqkwvrdz5w6w2y372508q30rlnl30rzccczqjwwqq7"
}
```
2. Transfer capacity to this address, if the address have enough capacity, you can skip this step. Be careful about the `--skip-check-to-address` parameter, please double check the receiver's address by yourself.
```bash
ckb-cli wallet transfer --from-account 0xb398368a8ed39448f95479c1178ff3fc5e316318 \
  --to-address ckt1qqhj387qfxcxsa7xthycgf8q5k5yl8n2rfeacyn258euza9shmfsgqgqkwvrdz5w6w2y372508q30rlnl30rzccczqjwwqq7 \
  --capacity 99 --skip-check-to-address
```
In this example, i got the transaction:
`0xf9b8f081f7823f0d6fecfb89e5ddd19699b705ca008e11b5a930026dd47bf896`
3. Get live cells of the address
```sh
ckb-cli wallet get-live-cells --address ckt1qqhj387qfxcxsa7xthycgf8q5k5yl8n2rfeacyn258euza9shmfsgqgqkwvrdz5w6w2y372508q30rlnl30rzccczqjwwqq7
```

4. Generate an open transaction willing to pay one CKB without fee
```bash
omnilock-cli generate-tx pubkey-hash \
  --pubkey-hash 0xb398368a8ed39448f95479c1178ff3fc5e316318 \
  --capacity 98 --open-capacity 1.0  --fee-rate 0 \
  --receiver ckt1qyqy68e02pll7qd9m603pqkdr29vw396h6dq50reug \
  --tx-file tx.json
```
In the generate `tx.json`, you will see the output capacity is `0x248202200` which is 9800000000 Shannon and is 98 CKB,
```json
...
"outputs": [
  {
    "capacity": "0x248202200",
    "lock": {
      "code_hash": "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8",
      "hash_type": "type",
      "args": "0x4d1f2f507fff01a5de9f1082cd1a8ac744babe9a"
    },
    "type": null
  }
],
...
```
3. Sign the open transaction
```sh
omnilock-cli sign pubkey-hash --tx-file tx.json --from-account b398368a8ed39448f95479c1178ff3fc5e316318
```
4. Add input cell, the cell get 142.9 CKB
```sh
omnilock-cli add-input --tx-hash 0x28b312399ce4fb35358c49fd21cd8a9422949014aa05ec0dc5f31752de70bd79 --index 0 --tx-file tx.json
```
5. Add output cell, the --capacity is 143.89997887, so the tx_fee is 0.00002113 CKB.
```sh
./target/debug/omnilock-cli add-output --to-address ckt1qyqz7xmq3ee2nfu4k04thv4vuczd3tqt465qtjdy89 --capacity 143.89997887 --tx-file tx.json
```
6. Transfer the tx-file into ckb-cli compatible version
  - Manually transfer the file:
    * Copy the tx.json to tx.ckb-cli.json
    * Edit the tx.ckb-cli.json, replace `"omnilock_config"` section with `"multisig_configs": {},  "signatures": {}`.

7. Sign the transaction with ckb-cli, because the ckb-cli have no idea about omnilock, so we add `--skip-check` parameter, be careful about this parameter.
```sh
ckb-cli tx sign-inputs --from-account 0x2f1b608e72a9a795b3eabbb2ace604d8ac0baea8 --tx-file tx.ckb-cli.json --skip-check
```
It will only print the signature and it's according lock-arg:
```yaml
- lock-arg: 0x2f1b608e72a9a795b3eabbb2ace604d8ac0baea8
  signature: 0xe80837cfc045519f7d9d94ddf460c8ad8a43fc150f494cfca7cb650706e7ad0731ca2a2d41d463a13210fb75186e72a68d8e774bf96ed845648b7f700970e30d00
```
8. Add the signature
```sh
ckb-cli tx add-signature --lock-arg 0x2f1b608e72a9a795b3eabbb2ace604d8ac0baea8 \
  --signature 0xe80837cfc045519f7d9d94ddf460c8ad8a43fc150f494cfca7cb650706e7ad0731ca2a2d41d463a13210fb75186e72a68d8e774bf96ed845648b7f700970e30d00 \
  --tx-file tx.ckb-cli.json
```
9.  Send the transaction, we add `--skip-check` to avoid the `invalid lock script code_hash` complain, since ckb-cli known nothing about it, please be careful.
```sh
tx send --tx-file tx.ckb-cli.json --skip-check
```

### Ethereum open transaction.
1. Build a sighash opentx address.
. Generate the transaction
```sh
omnilock-cli build-address ethereum --ethereum-address 0xcf2485c76aff1f2b4464edf04a1c8045068cf7e0 --flags opentx
```
Here is the example output,
```json
{
  "lock-arg": "0x01cf2485c76aff1f2b4464edf04a1c8045068cf7e010",
  "lock-hash": "0xac737a042e05e0f8379886e2a4f8d4e5ae9fd97269f5f433df7064a789045c81",
  "mainnet": "ckb1qqhj387qfxcxsa7xthycgf8q5k5yl8n2rfeacyn258euza9shmfsgqgpeujgt3m2lu0jk3ryahcy58yqg5rgealqzqhutx75",
  "testnet": "ckt1qqhj387qfxcxsa7xthycgf8q5k5yl8n2rfeacyn258euza9shmfsgqgpeujgt3m2lu0jk3ryahcy58yqg5rgealqzqvtl2lu"
}
```
2. Sign the transaction

### Multisig open transaction.
1. Build a sighash opentx address.
. Generate the transaction
```bash
omnilock-cli build-address multisig --require-first-n 0 --threshold 2 \
                                    --sighash-address ckt1qyqt8xpk328d89zgl928nsgh3lelch33vvvq5u3024 \
                                    --sighash-address ckt1qyqvsv5240xeh85wvnau2eky8pwrhh4jr8ts8vyj37 \
                                    --sighash-address ckt1qyqywrwdchjyqeysjegpzw38fvandtktdhrs0zaxl4 \
                                    --flags opentx
```
Here is the example output,
```json
{
  "lock-arg": "0x065d7d0128eeaa6f9656a229b42aadd0b177d387eb10",
  "lock-hash": "0xc0a5df548c36db22ccb14b283b101b352d0d3d9921d5ccba067ef183e28417f5",
  "mainnet": "ckb1qqhj387qfxcxsa7xthycgf8q5k5yl8n2rfeacyn258euza9shmfsgqgxt47sz28w4fhev44z9x6z4twsk9ma8pltzq74wfye",
  "testnet": "ckt1qqhj387qfxcxsa7xthycgf8q5k5yl8n2rfeacyn258euza9shmfsgqgxt47sz28w4fhev44z9x6z4twsk9ma8pltzq9z6993"
}
```
2. Sign the transaction
