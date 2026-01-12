#!/usr/bin/env python3
"""
CGI Test Script - Python
Displays all CGI environment variables and request information
"""

import os
import sys
from datetime import datetime

# Print HTTP headers
print("Content-Type: text/html")
print()

# HTML output
print("""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CGI Test - Python</title>
    <style>
        body {
            font-family: 'Courier New', monospace;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
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
            color: #667eea;
            border-bottom: 3px solid #667eea;
            padding-bottom: 10px;
        }
        h2 {
            color: #764ba2;
            margin-top: 30px;
        }
        .info-box {
            background: #f8f9fa;
            border-left: 4px solid #667eea;
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
        .timestamp {
            color: #6c757d;
            font-size: 12px;
        }
        a {
            color: #667eea;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🐍 CGI Test Script - Python</h1>
        <div class="info-box">
            <p class="success">✓ CGI script executed successfully!</p>
            <p class="timestamp">Generated: """ + datetime.now().strftime("%Y-%m-%d %H:%M:%S") + """</p>
        </div>
        
        <h2>📋 CGI Environment Variables</h2>
""")

# Display all environment variables
env_vars = [
    'REQUEST_METHOD', 'PATH_INFO', 'QUERY_STRING', 'CONTENT_TYPE',
    'CONTENT_LENGTH', 'SERVER_NAME', 'SERVER_PORT', 'SERVER_PROTOCOL',
    'SCRIPT_NAME', 'REMOTE_ADDR', 'REMOTE_HOST', 'HTTP_HOST',
    'HTTP_USER_AGENT', 'HTTP_ACCEPT', 'HTTP_COOKIE'
]

for var in env_vars:
    value = os.environ.get(var, '<not set>')
    print(f'<div class="env-var"><strong>{var}:</strong> {value}</div>')

# Display all other environment variables
print("<h2>🔧 All Environment Variables</h2>")
for key in sorted(os.environ.keys()):
    if key not in env_vars:
        value = os.environ.get(key, '')
        print(f'<div class="env-var"><strong>{key}:</strong> {value}</div>')

# Display POST data if available
if os.environ.get('REQUEST_METHOD') == 'POST':
    content_length = int(os.environ.get('CONTENT_LENGTH', 0))
    if content_length > 0:
        post_data = sys.stdin.read(content_length)
        print(f"""
        <h2>📤 POST Data</h2>
        <div class="info-box">
            <pre>{post_data}</pre>
        </div>
        """)

print("""
        <h2>🔗 Navigation</h2>
        <div class="info-box">
            <p><a href="/">← Back to Home</a></p>
            <p><a href="/cgi-bin/test.sh">Test Shell CGI</a></p>
            <p><a href="/cgi-bin/test.pl">Test Perl CGI</a></p>
        </div>
    </div>
</body>
</html>
""")
