#!/usr/bin/env python3

import random
import sys

if len(sys.argv) != 2:
        print("Usage: generate.py <num_records>")
        sys.exit(1)

num_records = int(sys.argv[1])

print('type,client,tx,amount')

# Lookup table for deposit and withdrawal
basic_transactions = {
    'deposit': lambda clientId, txId, amount: f"deposit,{clientId},{txId},{amount}",
    'withdraw': lambda clientId, txId, amount: f"withdrawal,{clientId},{txId},{amount}"
}

client_id = 0
max_client_id = 0

disputed_tx_ids = []
output = []

for tx_id in range(num_records):
    # Generate random deposit or withdrawal
    basic_transaction = random.choice(list(basic_transactions.values()))
    if random.random() < 0.75:
        max_client_id = max(max_client_id, client_id + 1)
        client_id += 1
    else:
        client_id = random.randint(0, max_client_id)

    output.append(basic_transaction(client_id, tx_id, round((random.random() + 1) * 100, 4)))

    # Once in a while, generate a dispute
    if random.random() < 0.1:
        output.append(f"dispute,{client_id},{tx_id}")
        disputed_tx_ids.append(tx_id)

    # Resolve a dispute occasionally
    if disputed_tx_ids and random.random() < 0.1:
        dispute_index = random.randint(0, len(disputed_tx_ids) - 1)
        dispute_tx_id = disputed_tx_ids.pop(dispute_index)
        output.append(f"resolve,{client_id},{dispute_tx_id}")

    # Issue a chargeback occasionally
    if disputed_tx_ids and random.random() < 0.1:
        dispute_index = random.randint(0, len(disputed_tx_ids) - 1)
        dispute_tx_id = disputed_tx_ids.pop(dispute_index)
        output.append(f"chargeback,{client_id},{dispute_tx_id}")

# Print the output
print("\n".join(output))
