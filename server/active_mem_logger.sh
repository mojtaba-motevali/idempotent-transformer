#!/usr/bin/env bash
# avg_mem_logger.sh
# Logs *average* memory usage (approximated by Referenced)
# for processes whose command contains 'idempotency', every 1 second.

PATTERN="idempotency"
INTERVAL=1
LOGFILE="avg_mem.csv"

# Associative arrays for totals & counts
declare -A total_usage
declare -A sample_count

# Write CSV header if needed
if [ ! -f "$LOGFILE" ]; then
  echo "timestamp,pid,comm,avg_mem_usage_kb" > "$LOGFILE"
fi

trap 'echo "Exiting..."; exit 0' INT TERM

while true; do
  PIDS=$(pgrep -f "$PATTERN")
  TS=$(date '+%Y-%m-%d %H:%M:%S')

  # Clear referenced bits per PID
  for pid in $PIDS; do
    if [ -w "/proc/$pid/clear_refs" ]; then
      echo 1 > "/proc/$pid/clear_refs" 2>/dev/null || true
    fi
  done

  sleep "$INTERVAL"

  echo "==== $TS ===="
  for pid in $PIDS; do
    [ -d "/proc/$pid" ] || continue

    if [ -r "/proc/$pid/comm" ]; then
      comm=$(tr -d '\0' < "/proc/$pid/comm")
    else
      comm="$PATTERN"
    fi

    mem=$(awk '/^Referenced:/ {print $2; exit}' "/proc/$pid/smaps_rollup" 2>/dev/null)
    if [ -z "$mem" ]; then
      mem=$(awk '/^Referenced:/ {s+=$2} END{print s+0}' "/proc/$pid/smaps" 2>/dev/null)
    fi
    mem=${mem:-0}

    # Update running totals
    total_usage[$pid]=$(( ${total_usage[$pid]:-0} + mem ))
    sample_count[$pid]=$(( ${sample_count[$pid]:-0} + 1 ))

    avg=$(( ${total_usage[$pid]} / ${sample_count[$pid]} ))

    printf "PID: %s | COMM: %s | Avg Mem Usage: %s kB\n" \
      "$pid" "$comm" "$avg"

    echo "$TS,$pid,$comm,$avg" >> "$LOGFILE"
  done

  echo
done
