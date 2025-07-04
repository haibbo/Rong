#!/bin/bash

# Rong Test Runner
# Supports running tests across all JavaScript engines

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Available engines
ENGINES=("quickjs" "jscore")

# Auto-discover test categories
get_core_tests() {
    if [[ -d "tests" ]]; then
        find tests -name "*.rs" -type f | sed 's|tests/||; s|\.rs$||' | sort
    fi
}

get_module_tests() {
    if [[ -d "modules" ]]; then
        find modules -maxdepth 1 -type d | sed 's|modules/||' | grep -v '^$' | grep -v '^modules$' | grep -v '^\.' | sort
    fi
}

# Initialize test arrays
CORE_TESTS=($(get_core_tests))
MODULE_TESTS=($(get_module_tests))

# Statistics
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}  Rong Test Runner${NC}"
    echo -e "${BLUE}================================${NC}"
}

print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -e, --engine ENGINE     Run tests for specific engine (quickjs, jscore, all)"
    echo "  -t, --test TEST         Run specific test (core test name or module name)"
    echo "  -c, --core              Run only core tests"
    echo "  -m, --modules           Run only module tests"
    echo "  -h, --help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                      # Run all tests on all engines"
    echo "  $0 -e quickjs          # Run all tests on QuickJS"
    echo "  $0 -e jscore -c        # Run core tests on JavaScriptCore"
    echo "  $0 -t iterator         # Run iterator tests on all engines"
    echo "  $0 -t rong_http        # Run rong_http module tests on all engines"
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

run_core_test() {
    local test_name=$1
    local engine=$2

    log_info "Running core test: $test_name (engine: $engine)"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    if cargo test --test="$test_name" --features="$engine" --quiet; then
        log_success "Core test $test_name passed on $engine"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        log_error "Core test $test_name failed on $engine"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

run_module_test() {
    local module_name=$1
    local engine=$2

    log_info "Running module test: $module_name (engine: $engine)"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))

    if cargo test -p "$module_name" --features="$engine" --quiet; then
        log_success "Module test $module_name passed on $engine"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        log_error "Module test $module_name failed on $engine"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

run_all_core_tests() {
    local engine=$1

    echo -e "\n${YELLOW}Running core tests on $engine...${NC}"

    for test in "${CORE_TESTS[@]}"; do
        run_core_test "$test" "$engine" || true
    done
}

run_all_module_tests() {
    local engine=$1

    echo -e "\n${YELLOW}Running module tests on $engine...${NC}"

    for module in "${MODULE_TESTS[@]}"; do
        run_module_test "$module" "$engine" || true
    done
}

run_specific_test() {
    local test_name=$1
    local engine=$2

    # Check if it's a core test
    for core_test in "${CORE_TESTS[@]}"; do
        if [[ "$core_test" == "$test_name" ]]; then
            run_core_test "$test_name" "$engine"
            return $?
        fi
    done

    # Check if it's a module test
    for module_test in "${MODULE_TESTS[@]}"; do
        if [[ "$module_test" == "$test_name" ]]; then
            run_module_test "$test_name" "$engine"
            return $?
        fi
    done

    log_error "Unknown test: $test_name"
    return 1
}

print_summary() {
    echo -e "\n${BLUE}================================${NC}"
    echo -e "${BLUE}  Test Summary${NC}"
    echo -e "${BLUE}================================${NC}"
    echo -e "Total tests: $TOTAL_TESTS"
    echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
    echo -e "${RED}Failed: $FAILED_TESTS${NC}"

    if [[ $FAILED_TESTS -eq 0 ]]; then
        echo -e "\n${GREEN}All tests passed! 🎉${NC}"
        exit 0
    else
        echo -e "\n${RED}Some tests failed! ❌${NC}"
        exit 1
    fi
}

# Parse command line arguments
ENGINE_FILTER=""
TEST_FILTER=""
CORE_ONLY=false
MODULES_ONLY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -e|--engine)
            ENGINE_FILTER="$2"
            shift 2
            ;;
        -t|--test)
            TEST_FILTER="$2"
            shift 2
            ;;
        -c|--core)
            CORE_ONLY=true
            shift
            ;;
        -m|--modules)
            MODULES_ONLY=true
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

# Validate engine filter
if [[ -n "$ENGINE_FILTER" && "$ENGINE_FILTER" != "all" ]]; then
    if [[ ! " ${ENGINES[@]} " =~ " ${ENGINE_FILTER} " ]]; then
        log_error "Unknown engine: $ENGINE_FILTER"
        echo "Available engines: ${ENGINES[*]}"
        exit 1
    fi
    ENGINES=("$ENGINE_FILTER")
fi

print_header

# Main execution
for engine in "${ENGINES[@]}"; do
    echo -e "\n${YELLOW}Testing with engine: $engine${NC}"

    if [[ -n "$TEST_FILTER" ]]; then
        # Run specific test
        run_specific_test "$TEST_FILTER" "$engine" || true
    elif [[ "$CORE_ONLY" == true ]]; then
        # Run only core tests
        run_all_core_tests "$engine"
    elif [[ "$MODULES_ONLY" == true ]]; then
        # Run only module tests
        run_all_module_tests "$engine"
    else
        # Run all tests
        run_all_core_tests "$engine"
        run_all_module_tests "$engine"
    fi
done

print_summary
