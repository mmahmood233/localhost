#!/bin/bash
# CGI Test Script - Shell
# Displays CGI environment variables

echo "Content-Type: text/html"
echo ""

cat << 'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CGI Test - Shell</title>
    <style>
        body {
            font-family: 'Courier New', monospace;
            background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
            padding: 20px;
            margin: 0;
        }
        .container {
            max-width: 1000px;
            margin: 0 auto;
            background: white;
            border-radius: 10px;
            padding: 30px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
        }
        h1 {
            color: #4facfe;
            border-bottom: 3px solid #4facfe;
            padding-bottom: 10px;
        }
        h2 {
            color: #00f2fe;
            margin-top: 30px;
        }
        .info-box {
            background: #f8f9fa;
            border-left: 4px solid #4facfe;
            padding: 15px;
            margin: 10px 0;
            border-radius: 5px;
        }
        .env-var {
            background: #e9ecef;
            padding: 10px;
            margin: 5px 0;
            border-radius: 3px;
            font-size: 14px;
        }
        .env-var strong {
            color: #495057;
            display: inline-block;
            min-width: 200px;
        }
        .success {
            color: #28a745;
            font-weight: bold;
        }
        a {
            color: #4facfe;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🐚 CGI Test Script - Shell</h1>
        <div class="info-box">
            <p class="success">✓ Shell CGI script executed successfully!</p>
            <p>Generated: $(date)</p>
        </div>
        
        <h2>📋 CGI Environment Variables</h2>
EOF

# Display key CGI variables
echo "<div class='env-var'><strong>REQUEST_METHOD:</strong> ${REQUEST_METHOD:-<not set>}</div>"
echo "<div class='env-var'><strong>PATH_INFO:</strong> ${PATH_INFO:-<not set>}</div>"
echo "<div class='env-var'><strong>QUERY_STRING:</strong> ${QUERY_STRING:-<not set>}</div>"
echo "<div class='env-var'><strong>CONTENT_TYPE:</strong> ${CONTENT_TYPE:-<not set>}</div>"
echo "<div class='env-var'><strong>CONTENT_LENGTH:</strong> ${CONTENT_LENGTH:-<not set>}</div>"
echo "<div class='env-var'><strong>SERVER_NAME:</strong> ${SERVER_NAME:-<not set>}</div>"
echo "<div class='env-var'><strong>SERVER_PORT:</strong> ${SERVER_PORT:-<not set>}</div>"
echo "<div class='env-var'><strong>SERVER_PROTOCOL:</strong> ${SERVER_PROTOCOL:-<not set>}</div>"
echo "<div class='env-var'><strong>SCRIPT_NAME:</strong> ${SCRIPT_NAME:-<not set>}</div>"
echo "<div class='env-var'><strong>REMOTE_ADDR:</strong> ${REMOTE_ADDR:-<not set>}</div>"
echo "<div class='env-var'><strong>HTTP_HOST:</strong> ${HTTP_HOST:-<not set>}</div>"
echo "<div class='env-var'><strong>HTTP_USER_AGENT:</strong> ${HTTP_USER_AGENT:-<not set>}</div>"

cat << 'EOF'
        <h2>🔧 All Environment Variables</h2>
EOF

# Display all environment variables
env | sort | while IFS='=' read -r key value; do
    echo "<div class='env-var'><strong>$key:</strong> $value</div>"
done

cat << 'EOF'
        <h2>🔗 Navigation</h2>
        <div class="info-box">
            <p><a href="/">← Back to Home</a></p>
            <p><a href="/cgi-bin/test.py">Test Python CGI</a></p>
            <p><a href="/cgi-bin/test.pl">Test Perl CGI</a></p>
        </div>
    </div>
</body>
</html>
EOF
