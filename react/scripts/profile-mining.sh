#!/bin/bash
# Profile cyb.app mining for memory leaks and CPU hotspots
# Usage: ./profile-mining.sh <PID> [duration_seconds]

PID=${1:-$(pgrep -f "cyb.app/Contents/MacOS/app" | head -1)}
DURATION=${2:-60}
INTERVAL=10
OUTDIR="/tmp/cyb-profile-$(date +%Y%m%d_%H%M%S)"

if [ -z "$PID" ]; then
    echo "ERROR: cyb.app not running. Pass PID as argument or launch the app first."
    exit 1
fi

mkdir -p "$OUTDIR"
echo "Profiling PID $PID for ${DURATION}s, sampling every ${INTERVAL}s"
echo "Output: $OUTDIR"
echo "---"

# Take initial snapshot
echo "[$(date +%H:%M:%S)] Initial snapshot..."
heap --sortBySize "$PID" > "$OUTDIR/heap_start.txt" 2>&1
vmmap --summary "$PID" > "$OUTDIR/vmmap_start.txt" 2>&1
leaks --noContent "$PID" > "$OUTDIR/leaks_start.txt" 2>&1

# Record memory every INTERVAL seconds
echo "timestamp,rss_mb,virt_mb,cpu_pct" > "$OUTDIR/memory_timeline.csv"
ELAPSED=0
while [ $ELAPSED -lt $DURATION ]; do
    STATS=$(ps -o rss=,vsz=,%cpu= -p "$PID" 2>/dev/null)
    if [ -z "$STATS" ]; then
        echo "[$(date +%H:%M:%S)] Process exited!"
        break
    fi
    RSS=$(echo "$STATS" | awk '{print $1/1024}')
    VSZ=$(echo "$STATS" | awk '{print $2/1024}')
    CPU=$(echo "$STATS" | awk '{print $3}')
    echo "$(date +%H:%M:%S),$RSS,$VSZ,$CPU" >> "$OUTDIR/memory_timeline.csv"
    echo "[$(date +%H:%M:%S)] RSS: ${RSS}MB  CPU: ${CPU}%"
    sleep $INTERVAL
    ELAPSED=$((ELAPSED + INTERVAL))
done

# Take final snapshot
echo "[$(date +%H:%M:%S)] Final snapshot..."
heap --sortBySize "$PID" > "$OUTDIR/heap_end.txt" 2>&1
vmmap --summary "$PID" > "$OUTDIR/vmmap_end.txt" 2>&1
leaks --noContent "$PID" > "$OUTDIR/leaks_end.txt" 2>&1
sample "$PID" 5 > "$OUTDIR/cpu_sample.txt" 2>&1

# Diff heap
echo ""
echo "=== HEAP DIFF (top growers) ==="
START_BYTES=$(grep "total" "$OUTDIR/heap_start.txt" | grep -oE '[0-9]+ bytes' | head -1)
END_BYTES=$(grep "total" "$OUTDIR/heap_end.txt" | grep -oE '[0-9]+ bytes' | head -1)
echo "Start: $START_BYTES"
echo "End:   $END_BYTES"

echo ""
echo "=== LEAK DIFF ==="
START_LEAKS=$(grep "leaks for" "$OUTDIR/leaks_start.txt" | head -1)
END_LEAKS=$(grep "leaks for" "$OUTDIR/leaks_end.txt" | head -1)
echo "Start: $START_LEAKS"
echo "End:   $END_LEAKS"

echo ""
echo "=== MEMORY TIMELINE ==="
cat "$OUTDIR/memory_timeline.csv"

echo ""
echo "Full reports in: $OUTDIR"
