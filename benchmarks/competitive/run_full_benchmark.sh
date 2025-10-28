#!/bin/bash
#
# Full Benchmark Automation Script
#
# This script automates the entire benchmark workflow:
# 1. Checks prerequisites
# 2. Runs benchmarks
# 3. Generates reports
#

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DATABASES="${DATABASES:-dvecdb qdrant}"
DATASETS="${DATASETS:-small}"
SKIP_CHECKS="${SKIP_CHECKS:-false}"

echo -e "${BLUE}================================================================================${NC}"
echo -e "${BLUE}d-vecDB Competitive Benchmark Suite${NC}"
echo -e "${BLUE}================================================================================${NC}"
echo ""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --databases)
            DATABASES="$2"
            shift 2
            ;;
        --datasets)
            DATASETS="$2"
            shift 2
            ;;
        --skip-checks)
            SKIP_CHECKS=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --databases DB1 DB2    Databases to benchmark (default: dvecdb qdrant)"
            echo "                         Options: dvecdb, pinecone, qdrant"
            echo "  --datasets DS1 DS2     Datasets to use (default: small)"
            echo "                         Options: small, medium, large, realistic"
            echo "  --skip-checks          Skip prerequisite checks"
            echo "  --help                 Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  PINECONE_API_KEY       Required if benchmarking Pinecone"
            echo ""
            echo "Examples:"
            echo "  $0"
            echo "  $0 --databases dvecdb qdrant --datasets realistic"
            echo "  $0 --databases dvecdb pinecone qdrant --datasets small medium"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "Configuration:"
echo "  Databases: $DATABASES"
echo "  Datasets: $DATASETS"
echo ""

# Step 1: Check prerequisites
if [ "$SKIP_CHECKS" = false ]; then
    echo -e "${YELLOW}Step 1: Checking prerequisites...${NC}"
    echo ""

    # Check Python
    if ! command -v python3 &> /dev/null; then
        echo -e "${RED}❌ Python 3 not found${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Python 3 found:${NC} $(python3 --version)"

    # Check pip
    if ! command -v pip &> /dev/null && ! command -v pip3 &> /dev/null; then
        echo -e "${RED}❌ pip not found${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ pip found${NC}"

    # Check if in virtual environment (recommended)
    if [ -z "$VIRTUAL_ENV" ]; then
        echo -e "${YELLOW}⚠️  Not in a virtual environment (recommended but not required)${NC}"
    else
        echo -e "${GREEN}✅ Using virtual environment:${NC} $VIRTUAL_ENV"
    fi

    # Check d-vecDB server
    if [[ "$DATABASES" == *"dvecdb"* ]]; then
        if ! curl -s http://localhost:8080/health > /dev/null 2>&1; then
            echo -e "${RED}❌ d-vecDB server not running at localhost:8080${NC}"
            echo -e "${YELLOW}   Start with: ./target/release/vectordb-server --host 0.0.0.0 --rest-port 8080${NC}"
            exit 1
        fi
        echo -e "${GREEN}✅ d-vecDB server is running${NC}"
    fi

    # Check Qdrant
    if [[ "$DATABASES" == *"qdrant"* ]]; then
        if ! curl -s http://localhost:6333/collections > /dev/null 2>&1; then
            echo -e "${RED}❌ Qdrant not running at localhost:6333${NC}"
            echo -e "${YELLOW}   Start with: docker run -d -p 6333:6333 qdrant/qdrant${NC}"
            exit 1
        fi
        echo -e "${GREEN}✅ Qdrant is running${NC}"
    fi

    # Check Pinecone API key
    if [[ "$DATABASES" == *"pinecone"* ]]; then
        if [ -z "$PINECONE_API_KEY" ]; then
            echo -e "${RED}❌ PINECONE_API_KEY environment variable not set${NC}"
            echo -e "${YELLOW}   Set with: export PINECONE_API_KEY=your_key_here${NC}"
            exit 1
        fi
        echo -e "${GREEN}✅ Pinecone API key is set${NC}"
    fi

    # Check Python dependencies
    echo ""
    echo "Checking Python dependencies..."

    if ! python3 -c "import numpy" 2>/dev/null; then
        echo -e "${RED}❌ numpy not installed${NC}"
        echo -e "${YELLOW}   Install with: pip install -r requirements.txt${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ numpy installed${NC}"

    if ! python3 -c "import pandas" 2>/dev/null; then
        echo -e "${YELLOW}⚠️  pandas not installed (optional)${NC}"
    else
        echo -e "${GREEN}✅ pandas installed${NC}"
    fi

    if ! python3 -c "import matplotlib" 2>/dev/null; then
        echo -e "${YELLOW}⚠️  matplotlib not installed (required for report generation)${NC}"
    else
        echo -e "${GREEN}✅ matplotlib installed${NC}"
    fi

    if [[ "$DATABASES" == *"dvecdb"* ]]; then
        if ! python3 -c "import vectordb_client" 2>/dev/null; then
            echo -e "${RED}❌ vectordb_client not installed${NC}"
            echo -e "${YELLOW}   Install with: cd ../../python-client && pip install -e .${NC}"
            exit 1
        fi
        echo -e "${GREEN}✅ vectordb_client installed${NC}"
    fi

    if [[ "$DATABASES" == *"qdrant"* ]]; then
        if ! python3 -c "import qdrant_client" 2>/dev/null; then
            echo -e "${RED}❌ qdrant_client not installed${NC}"
            echo -e "${YELLOW}   Install with: pip install -r requirements.txt${NC}"
            exit 1
        fi
        echo -e "${GREEN}✅ qdrant_client installed${NC}"
    fi

    if [[ "$DATABASES" == *"pinecone"* ]]; then
        if ! python3 -c "import pinecone" 2>/dev/null; then
            echo -e "${RED}❌ pinecone-client not installed${NC}"
            echo -e "${YELLOW}   Install with: pip install -r requirements.txt${NC}"
            exit 1
        fi
        echo -e "${GREEN}✅ pinecone-client installed${NC}"
    fi

    echo ""
    echo -e "${GREEN}✅ All prerequisites met!${NC}"
    echo ""
else
    echo -e "${YELLOW}Skipping prerequisite checks...${NC}"
    echo ""
fi

# Step 2: Run benchmarks
echo -e "${YELLOW}Step 2: Running benchmarks...${NC}"
echo ""
echo "This may take several minutes depending on dataset size."
echo ""

START_TIME=$(date +%s)

python3 run_benchmarks.py \
    --databases $DATABASES \
    --datasets $DATASETS

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
echo -e "${GREEN}✅ Benchmarks completed in ${DURATION}s${NC}"
echo ""

# Step 3: Generate report
echo -e "${YELLOW}Step 3: Generating visualization report...${NC}"
echo ""

# Find the most recent results file
RESULTS_FILE=$(ls -t results/benchmark_results_*.json 2>/dev/null | head -1)

if [ -z "$RESULTS_FILE" ]; then
    echo -e "${RED}❌ No results file found${NC}"
    exit 1
fi

echo "Results file: $RESULTS_FILE"
echo ""

if ! python3 -c "import matplotlib" 2>/dev/null; then
    echo -e "${YELLOW}⚠️  matplotlib not installed, skipping chart generation${NC}"
    echo -e "${YELLOW}   Install with: pip install matplotlib seaborn${NC}"
else
    python3 generate_report.py "$RESULTS_FILE"
fi

echo ""
echo -e "${BLUE}================================================================================${NC}"
echo -e "${GREEN}🎉 BENCHMARK COMPLETE!${NC}"
echo -e "${BLUE}================================================================================${NC}"
echo ""
echo "Results:"
echo "  📊 JSON Results: $RESULTS_FILE"

if [ -f "results/benchmark_report.html" ]; then
    echo "  📈 HTML Report: results/benchmark_report.html"
    echo "  📉 Charts: results/plots/"
    echo ""
    echo "View the report:"

    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "  open results/benchmark_report.html"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "  xdg-open results/benchmark_report.html"
    else
        echo "  Open results/benchmark_report.html in your browser"
    fi
fi

echo ""
echo "Quick analysis:"
echo "  cat $RESULTS_FILE | jq '.[] | select(.operation==\"search\" and .metadata.top_k==10) | {database, latency_p50, throughput}'"
echo ""
echo -e "${GREEN}Happy benchmarking! 🚀${NC}"
