#!/bin/bash


### IMPORTANT:  THIS SCRIPT IS OUTDATED AND SHOULD NOT BE USED. IT WAS USED TO TEST THINGS RELATED TO CORE PINNING, BUT
###             FOR THE CURRENT SCOPE THIS DOES NOT PROVIDE ANY PERFORMANCE IMPROVEMENTS AND THEREFORE ONLY ADDS OVERHEAD
cleanup() {
  echo "Tearing down shield..."
  sudo systemctl set-property --runtime user.slice AllowedCPUs=
  sudo systemctl set-property --runtime system.slice AllowedCPUs=
  sudo systemctl stop irqbalance
  echo "Done: Cores returned to host system."

  echo "Shutting down VM..."
  sudo virsh shutdown orderbook-vm
  echo "Done: VM shutdown successfully."

  echo "-------------------------------------------------------"
}

run_benchmark() {
  OS_CORES="0-3,8-15"

  echo "-------------------------------------------------------"
  echo "ORDERBOOK BENCHMARK RUNNER"
  echo "-------------------------------------------------------"

  echo "Starting VM..."
  if sudo virsh start orderbook-vm; then
    echo "Success: VM start signal sent."
    
    echo "Waiting for SSH to become available..."
    MAX_RETRIES=30
    COUNT=0
    until ssh -o ConnectTimeout=2 -o BatchMode=yes orderbook-vm "exit" > /dev/null 2>&1; do
        if [ $COUNT -ge $MAX_RETRIES ]; then
            echo "Error: VM timed out after 60 seconds."
            cleanup
            exit 1
        fi
        printf "."
        sleep 2
        ((COUNT++))
    done
    echo -e "Success: VM is reachable via SSH."
    
  else
    echo "Error: Failed to start VM."
    cleanup
    exit 1
  fi

  echo "Shielding Cores 4-7 by restricting OS to $OS_CORES..."
  if 
    sudo systemctl set-property --runtime user.slice AllowedCPUs=$OS_CORES \
    && sudo systemctl set-property --runtime system.slice AllowedCPUs=$OS_CORES \
    && sudo systemctl start irqbalance; then
    echo "Success: Cores 4-7 isolated from host OS."
  else
    echo "Error: Failed to set CPU affinity via systemd."
    cleanup
    exit 1
  fi

  echo "-------------------------------------------------------"
  ssh -T orderbook-vm << EOF
    cd /home/chuckie/orderbook
    RUSTFLAGS='-Awarnings' cargo bench -p benchmarks
EOF

  cleanup
}

    # RUSTFLAGS='-Awarnings' cargo bench -p benchmarks --features naive,standard,standard-arena
LOG_FILE="benchmark_$(date +%Y%m%d_%H%M%S).log"

run_benchmark 2>&1 | tee "$LOG_FILE"

echo "Log saved to: $LOG_FILE"
