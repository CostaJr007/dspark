#!/bin/bash
# Extrai dados de scaling para plotar gráfico

echo "N,O_Nk,O_N2,savings_pct"
for N in 3 5 10 20 50 100; do
    K=3
    ONK=$((N + (N - K) * K + K * (K - 1) / 2))
    ON2=$((N * (N - 1) / 2))
    SAVINGS=$(echo "scale=2; (1 - $ONK / $ON2) * 100" | bc)
    echo "$N,$ONK,$ON2,$SAVINGS"
done
