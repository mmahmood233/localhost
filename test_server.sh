#!/bin/bash
# Comprehensive Server Test Script
# Tests all implemented features

echo "🧪 Localhost HTTP Server - Comprehensive Test Suite"
echo "=================================================="
echo ""

SERVER_URL="http://127.0.0.1:8080"
PASS=0
FAIL=0

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test function
test_endpoint() {
    local name="$1"
    local method="$2"
    local endpoint="$3"
    local expected_code="$4"
    local extra_args="$5"
    
    echo -n "Testing $name... "
    
    if [ -z "$extra_args" ]; then
        response=$(curl -s -o /dev/null -w "%{http_code}" -X "$method" "$SERVER_URL$endpoint")
    else
        response=$(curl -s -o /dev/null -w "%{http_code}" -X "$method" $extra_args "$SERVER_URL$endpoint")
    fi
    
    if [ "$response" == "$expected_code" ]; then
        echo -e "${GREEN}✓ PASS${NC} (HTTP $response)"
        ((PASS++))
    else
        echo -e "${RED}✗ FAIL${NC} (Expected $expected_code, got $response)"
        ((FAIL++))
    fi
}

echo "📋 Test 1: Static File Serving"
echo "-------------------------------"
test_endpoint "Homepage" "GET" "/" "200"
test_endpoint "Plain text file" "GET" "/hello.txt" "200"
test_endpoint "JSON file" "GET" "/test.json" "200"
test_endpoint "Test upload file" "GET" "/test_upload.txt" "200"
echo ""

echo "🔍 Test 2: Error Pages"
echo "----------------------"
test_endpoint "404 Not Found" "GET" "/nonexistent-file.html" "404"
test_endpoint "405 Method Not Allowed" "POST" "/hello.txt" "405"
echo ""

echo "📤 Test 3: File Upload (POST)"
echo "-----------------------------"
# Create a test file
echo "Test upload content" > /tmp/test_upload.txt
test_endpoint "File upload" "POST" "/upload" "200" "-F 'file=@/tmp/test_upload.txt'"
rm -f /tmp/test_upload.txt
echo ""

echo "🗑️  Test 4: File Deletion (DELETE)"
echo "----------------------------------"
test_endpoint "Delete file" "DELETE" "/uploads/test.txt" "200"
echo ""

echo "🍪 Test 5: Cookie & Session Management"
echo "--------------------------------------"
test_endpoint "Create session" "GET" "/session/create" "200"
test_endpoint "Session info" "GET" "/session/info" "200"
test_endpoint "Session stats" "GET" "/session/stats" "200"
echo ""

echo "🐍 Test 6: CGI Scripts"
echo "---------------------"
test_endpoint "Python CGI" "GET" "/cgi-bin/test.py" "200"
test_endpoint "Shell CGI" "GET" "/cgi-bin/test.sh" "200"
test_endpoint "Perl CGI" "GET" "/cgi-bin/test.pl" "200"
echo ""

echo "🔀 Test 7: HTTP Redirects"
echo "------------------------"
test_endpoint "301 Redirect" "GET" "/redirect/301/home" "301"
test_endpoint "302 Redirect" "GET" "/redirect/302/home" "302"
test_endpoint "307 Redirect" "GET" "/redirect/307/home" "307"
test_endpoint "308 Redirect" "GET" "/redirect/308/home" "308"
echo ""

echo "📄 Test 8: Custom Pages"
echo "-----------------------"
test_endpoint "Upload page" "GET" "/upload.html" "200"
test_endpoint "Redirect test page" "GET" "/redirect-test.html" "200"
echo ""

echo "🎯 Test 9: HEAD Method"
echo "---------------------"
test_endpoint "HEAD request" "HEAD" "/" "200"
test_endpoint "HEAD on file" "HEAD" "/hello.txt" "200"
echo ""

echo "⚡ Test 10: HTTP/1.1 Features"
echo "----------------------------"
echo -n "Testing Keep-Alive... "
response=$(curl -s -I "$SERVER_URL/" | grep -i "Connection: keep-alive")
if [ -n "$response" ]; then
    echo -e "${GREEN}✓ PASS${NC}"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}"
    ((FAIL++))
fi

echo -n "Testing Server header... "
response=$(curl -s -I "$SERVER_URL/" | grep -i "Server:")
if [ -n "$response" ]; then
    echo -e "${GREEN}✓ PASS${NC} ($response)"
    ((PASS++))
else
    echo -e "${RED}✗ FAIL${NC}"
    ((FAIL++))
fi
echo ""

echo "=================================================="
echo "📊 Test Results Summary"
echo "=================================================="
echo -e "Total Tests: $((PASS + FAIL))"
echo -e "${GREEN}Passed: $PASS${NC}"
echo -e "${RED}Failed: $FAIL${NC}"

if [ $FAIL -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "\n${YELLOW}⚠️  Some tests failed. Please check the server implementation.${NC}"
    exit 1
fi
