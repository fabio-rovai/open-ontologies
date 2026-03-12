#!/bin/bash
# Download Java reasoner JARs for benchmark comparisons.
# Uses OWL API 4.5.29 + HermiT 1.4.3.456 + Pellet 2.4.0 (stable, compatible combo).
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

# OWL API 4.5.29 (distribution = all-in-one)
download "$MAVEN/net/sourceforge/owlapi/owlapi-distribution/4.5.29/owlapi-distribution-4.5.29.jar" \
    "$LIB_DIR/owlapi-distribution-4.5.29.jar"

# HermiT 1.4.3.456 (OWL API 4.x compatible)
download "$MAVEN/net/sourceforge/owlapi/org.semanticweb.hermit/1.4.3.456/org.semanticweb.hermit-1.4.3.456.jar" \
    "$LIB_DIR/HermiT-1.4.3.456.jar"

# Pellet 2.4.0-ignazio (OWL API 4.x compatible)
for mod in owlapi core rules datatypes el common query; do
    download "$MAVEN/com/clarkparsia/pellet/pellet-${mod}-ignazio/2.4.0-ignazio1.6.0/pellet-${mod}-ignazio-2.4.0-ignazio1.6.0.jar" \
        "$LIB_DIR/pellet-${mod}-ignazio-2.4.0-ignazio1.6.0.jar"
done

# Pellet's aterm dependency
download "$MAVEN/com/clarkparsia/pellet/aterm-java/1.8.2-p1/aterm-java-1.8.2-p1.jar" \
    "$LIB_DIR/aterm-java-1.8.2-p1.jar"

# SLF4J 1.7 (compatible with OWL API 4.x)
download "$MAVEN/org/slf4j/slf4j-api/1.7.36/slf4j-api-1.7.36.jar" \
    "$LIB_DIR/slf4j-api-1.7.36.jar"
download "$MAVEN/org/slf4j/slf4j-simple/1.7.36/slf4j-simple-1.7.36.jar" \
    "$LIB_DIR/slf4j-simple-1.7.36.jar"

# Guava 18 (Pellet 2.4's expected version)
download "$MAVEN/com/google/guava/guava/18.0/guava-18.0.jar" \
    "$LIB_DIR/guava-18.0.jar"

# Caffeine (OWL API 4.5.29 dependency)
download "$MAVEN/com/github/ben-manes/caffeine/caffeine/2.9.3/caffeine-2.9.3.jar" \
    "$LIB_DIR/caffeine-2.9.3.jar"

# javax.inject (needed on Java 17+)
download "$MAVEN/javax/inject/javax.inject/1/javax.inject-1.jar" \
    "$LIB_DIR/javax.inject-1.jar"

# JGraphT (Pellet dependency)
download "$MAVEN/org/jgrapht/jgrapht-core/0.9.0/jgrapht-core-0.9.0.jar" \
    "$LIB_DIR/jgrapht-core-0.9.0.jar"

# ANTLR runtime (HermiT dependency)
download "$MAVEN/org/antlr/antlr-runtime/3.5.3/antlr-runtime-3.5.3.jar" \
    "$LIB_DIR/antlr-runtime-3.5.3.jar"

echo ""
echo "All JARs in $LIB_DIR:"
ls "$LIB_DIR"
