#!/bin/bash
# Download OWL API, HermiT, and Pellet JARs for benchmark comparisons.
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LIB_DIR="$SCRIPT_DIR/lib"
mkdir -p "$LIB_DIR"

MAVEN="https://repo1.maven.org/maven2"

download() {
    local url="$1"
    local dest="$2"
    if [ -f "$dest" ]; then
        echo "  EXISTS: $(basename "$dest")"
        return
    fi
    echo "  DOWNLOAD: $(basename "$dest")"
    curl -fsSL -o "$dest" "$url"
}

echo "=== Setting up Java reasoner JARs ==="

# OWL API 5.1.20
download "$MAVEN/net/sourceforge/owlapi/owlapi-distribution/5.1.20/owlapi-distribution-5.1.20.jar" \
    "$LIB_DIR/owlapi-distribution-5.1.20.jar"

# HermiT 1.4.5.456 (OWL API-compatible build on Maven Central)
download "$MAVEN/net/sourceforge/owlapi/org.semanticweb.hermit/1.4.5.456/org.semanticweb.hermit-1.4.5.456.jar" \
    "$LIB_DIR/HermiT-1.4.5.456.jar"

# Openllet (actively maintained Pellet fork)
download "$MAVEN/com/github/galigator/openllet/openllet-owlapi/2.6.5/openllet-owlapi-2.6.5.jar" \
    "$LIB_DIR/openllet-owlapi-2.6.5.jar"
download "$MAVEN/com/github/galigator/openllet/openllet-core/2.6.5/openllet-core-2.6.5.jar" \
    "$LIB_DIR/openllet-core-2.6.5.jar"

# SLF4J (required by HermiT/Pellet)
download "$MAVEN/org/slf4j/slf4j-api/2.0.9/slf4j-api-2.0.9.jar" \
    "$LIB_DIR/slf4j-api-2.0.9.jar"
download "$MAVEN/org/slf4j/slf4j-simple/2.0.9/slf4j-simple-2.0.9.jar" \
    "$LIB_DIR/slf4j-simple-2.0.9.jar"

# Guava (Pellet dependency)
download "$MAVEN/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar" \
    "$LIB_DIR/guava-33.0.0-jre.jar"

echo ""
echo "All JARs in $LIB_DIR"
ls -la "$LIB_DIR"
