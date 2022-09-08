# Configuration
The configuration file points out which cell the omnilock script was deployed, the url of the ckb rpc and the ckb index url.

The default configuration file is ~/.omnilock.yaml, it can be specified with --config parameter.

## create an empty configure file.
- To init an empty configure file:
```bash
omnilock-cli config init
```
If the file already exist, you should remove it or rename it first.
- To init a differen empty configure file:
```bash
omnilock-cli -c omnilock.yaml config init
```

## fill the configure file with correct content

After create the empty file, modify the file, fill the correct content, so you can use the configure file in the following operation.

## check the configure file
- Check the default configuration file
```bash
omnilock-cli config check
```
-- Check the specified configuration file:
```bash
omnilock-cli config check
```

# build omnilock address
## build a pubkey-hash address with receiver
```bash
omnilock-cli build-address pubkey-hash --receiver ckt1qyqt8xpk328d89zgl928nsgh3lelch33vvvq5u3024
```
The output:
```json
{
  "lock-arg": "0x00b398368a8ed39448f95479c1178ff3fc5e31631800",
  "lock-hash": "0x6b845964aad7f568edf61a69d1c2278c68065dc91bad3c32234869aed86f7642",
  "mainnet": "ckb1qqklkz85v4xt39ws5dd2hdv8xsy4jnpe3envjzvddqecxr0mgvrksqgqkwvrdz5w6w2y372508q30rlnl30rzcccqq2pnflw",
  "testnet": "ckt1qqklkz85v4xt39ws5dd2hdv8xsy4jnpe3envjzvddqecxr0mgvrksqgqkwvrdz5w6w2y372508q30rlnl30rzcccqq3k897x"
}
```

## build a multisig address
```bash
omnilock-cli build-address multisig --require-first-n 0 --threshold 2 \
                                    --sighash-address ckt1qyqt8xpk328d89zgl928nsgh3lelch33vvvq5u3024 \
                                    --sighash-address ckt1qyqvsv5240xeh85wvnau2eky8pwrhh4jr8ts8vyj37 \
                                    --sighash-address ckt1qyqywrwdchjyqeysjegpzw38fvandtktdhrs0zaxl4
```
the output:
```json
{
  "lock-arg": "0x065d7d0128eeaa6f9656a229b42aadd0b177d387eb00",
  "lock-hash": "0xd93312782194cdb1a23dd73128795fd6a71ceb067ea9fd10546b95853d45f08e",
  "mainnet": "ckb1qqklkz85v4xt39ws5dd2hdv8xsy4jnpe3envjzvddqecxr0mgvrksqgxt47sz28w4fhev44z9x6z4twsk9ma8pltqqad8v6p",
  "testnet": "ckt1qqklkz85v4xt39ws5dd2hdv8xsy4jnpe3envjzvddqecxr0mgvrksqgxt47sz28w4fhev44z9x6z4twsk9ma8pltqqx6nqmf"
}
```
## build an ethereum address
1. build with receiver's private key:
```bash
omnilock-cli build-address ethereum --receiver-privkey 63d86723e08f0f813a36ce6aa123bb2289d90680ae1e99d4de8cdb334553f24d
```
the output:
```json
pubkey:"038d3cfceea4f9c2e76c5c4f5e99aec74c26d6ac894648b5700a0b71f91f9b5c2a"
pubkey:"048d3cfceea4f9c2e76c5c4f5e99aec74c26d6ac894648b5700a0b71f91f9b5c2a26b16aac1d5753e56849ea83bf795eb8b06f0b6f4e5ed7b8caca720595458039"
{
  "lock-arg": "0x01cf2485c76aff1f2b4464edf04a1c8045068cf7e000",
  "lock-hash": "0x04b791304bbd6287218acc9e4b0971789ea1ef52b758317481245913511c6159",
  "mainnet": "ckb1qqklkz85v4xt39ws5dd2hdv8xsy4jnpe3envjzvddqecxr0mgvrksqgpeujgt3m2lu0jk3ryahcy58yqg5rgealqqq5yzrqv",
  "testnet": "ckt1qqklkz85v4xt39ws5dd2hdv8xsy4jnpe3envjzvddqecxr0mgvrksqgpeujgt3m2lu0jk3ryahcy58yqg5rgealqqq0nk0py"
}
```
2. build with the compressed public key
```bash
omnilock-cli build-address ethereum --receiver-pubkey 038d3cfceea4f9c2e76c5c4f5e99aec74c26d6ac894648b5700a0b71f91f9b5c2a
```
3. build with the uncompressed public key
```bash
omnilock-cli build-address ethereum --receiver-pubkey 048d3cfceea4f9c2e76c5c4f5e99aec74c26d6ac894648b5700a0b71f91f9b5c2a26b16aac1d5753e56849ea83bf795eb8b06f0b6f4e5ed7b8caca720595458039
```

# auto transfer(draft)
This kind of transaction is suitable of unlock value of the cell.
1. build the address.
```bash
omnilock-cli build-address pubkey-hash --receiver ckt1qyqt8xpk328d89zgl928nsgh3lelch33vvvq5u3024
```
2. transfer capacity to this address(optional), be careful about the `--skip-check-to-address` parameter, double check the receiver's address.
```bash
 ckb-cli wallet transfer --from-account 0xc8328aabcd9b9e8e64fbc566c4385c3bdeb219d7 \
  --to-address ckt1qqklkz85v4xt39ws5dd2hdv8xsy4jnpe3envjzvddqecxr0mgvrksqgqkwvrdz5w6w2y372508q30rlnl30rzcccqq3k897x \
  --capacity 99 --skip-check-to-address
```
3. get live cells of the address
```bash
ckb-cli wallet get-live-cells --address ckt1qqklkz85v4xt39ws5dd2hdv8xsy4jnpe3envjzvddqecxr0mgvrksqgqkwvrdz5w6w2y372508q30rlnl30rzcccqq3k897x
```

4. generate transaction
```bash
# 8dadf1939b89919ca74b58fef41c0d4ec70cd6a7b093a0c8ca5b268f93b8181f is private key of address ckt1qyqt8xpk328d89zgl928nsgh3lelch33vvvq5u3024
omnilock-cli generate-tx --capacity 98.99999588 --receiver ckt1qyqy68e02pll7qd9m603pqkdr29vw396h6dq50reug --sender-key 8dadf1939b89919ca74b58fef41c0d4ec70cd6a7b093a0c8ca5b268f93b8181f --tx-file tx.json
```

5. sign the transaction
   
  - sign with according private key
  ```bash
  omnilock-cli sign --tx-file tx.json --sender-key 8dadf1939b89919ca74b58fef41c0d4ec70cd6a7b093a0c8ca5b268f93b8181f
  ```
  - sign the with according account
  ```bash
 omnilock-cli sign --tx-file tx.json --from-account b398368a8ed39448f95479c1178ff3fc5e316318
  ```
6. send the transaction
```bash
omnilock-cli send --tx-file tx.json
# >>> tx ac2cce746764cf9ecad7eefb82d24f8bcf5eb4708c65dde562bf96c86bbad831 sent! <<<
```
# manual transfer
## init empty transaction
## add input
## add output
## sign transaction
## send transaction