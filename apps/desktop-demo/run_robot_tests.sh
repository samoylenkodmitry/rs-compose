#!/bin/bash
# Run all robot tests for the desktop-demo application
# Usage: ./run_robot_tests.sh

cd "$(dirname "$0")"

PASS_COUNT=0
FAIL_COUNT=0

# List of automated robot tests
TESTS=(
    "robot_click_drag"
    "robot_tab_scroll"
    "robot_offset_test"
    "robot_tabs_scroll"
    "robot_async_tab_bug"
)

echo "========================================"
echo "         Robot Tests Runner"
echo "========================================"
echo ""

# Create a temporary file for capturing output
OUTPUT_FILE=$(mktemp)

cleanup() {
    rm -f "$OUTPUT_FILE"
}
trap cleanup EXIT

for test in "${TESTS[@]}"; do
    echo "--- Running: $test ---"
    
    # Run with 60 second timeout
    # Capture both stdout and stderr
    if timeout 60 cargo run --quiet --package desktop-app --example "$test" --features robot-app > "$OUTPUT_FILE" 2>&1; then
        EXIT_CODE=$?
    else
        EXIT_CODE=$?
    fi
    
    # Display the last few lines of output for context
    tail -n 10 "$OUTPUT_FILE"
    
    # Check for specific success marker in the output
    # We rely on the robot test printing "ALL TESTS PASSED"
    if grep -q "ALL TESTS PASSED" "$OUTPUT_FILE"; then
        echo "Result: ✓ PASS"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "Result: ✗ FAIL"
        if [ $EXIT_CODE -eq 124 ]; then
             echo "Reason: Timeout"
        else
             echo "Reason: Verification failed or crash (Exit code: $EXIT_CODE)"
        fi
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
    echo ""
done

echo "========================================"
echo "              SUMMARY"
echo "========================================"
echo "Passed: $PASS_COUNT"
echo "Failed: $FAIL_COUNT"
echo ""

if [ $FAIL_COUNT -gt 0 ]; then
    echo "✗ Some tests failed!"
    exit 1
else
    echo "✓ All tests passed!"
    exit 0
fi
