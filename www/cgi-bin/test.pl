#!/usr/bin/env perl
# CGI Test Script - Perl
# Displays CGI environment variables

use strict;
use warnings;

print "Content-Type: text/html\n\n";

my $timestamp = localtime();

print <<'HTML';
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CGI Test - Perl</title>
    <style>
        body {
            font-family: 'Courier New', monospace;
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
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
            color: #f093fb;
            border-bottom: 3px solid #f093fb;
            padding-bottom: 10px;
        }
        h2 {
            color: #f5576c;
            margin-top: 30px;
        }
        .info-box {
            background: #f8f9fa;
            border-left: 4px solid #f093fb;
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
            color: #f093fb;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🐪 CGI Test Script - Perl</h1>
        <div class="info-box">
            <p class="success">✓ Perl CGI script executed successfully!</p>
HTML

print "            <p>Generated: $timestamp</p>\n";

print <<'HTML';
        </div>
        
        <h2>📋 CGI Environment Variables</h2>
HTML

# Display key CGI variables
my @cgi_vars = qw(
    REQUEST_METHOD PATH_INFO QUERY_STRING CONTENT_TYPE CONTENT_LENGTH
    SERVER_NAME SERVER_PORT SERVER_PROTOCOL SCRIPT_NAME
    REMOTE_ADDR REMOTE_HOST HTTP_HOST HTTP_USER_AGENT HTTP_ACCEPT HTTP_COOKIE
);

foreach my $var (@cgi_vars) {
    my $value = $ENV{$var} || '<not set>';
    print "        <div class='env-var'><strong>$var:</strong> $value</div>\n";
}

print "        <h2>🔧 All Environment Variables</h2>\n";

# Display all environment variables
foreach my $key (sort keys %ENV) {
    next if grep { $_ eq $key } @cgi_vars;
    my $value = $ENV{$key} || '';
    print "        <div class='env-var'><strong>$key:</strong> $value</div>\n";
}

# Display POST data if available
if ($ENV{'REQUEST_METHOD'} eq 'POST' && $ENV{'CONTENT_LENGTH'}) {
    my $post_data;
    read(STDIN, $post_data, $ENV{'CONTENT_LENGTH'});
    print <<HTML;
        <h2>📤 POST Data</h2>
        <div class="info-box">
            <pre>$post_data</pre>
        </div>
HTML
}

print <<'HTML';
        <h2>🔗 Navigation</h2>
        <div class="info-box">
            <p><a href="/">← Back to Home</a></p>
            <p><a href="/cgi-bin/test.py">Test Python CGI</a></p>
            <p><a href="/cgi-bin/test.sh">Test Shell CGI</a></p>
        </div>
    </div>
</body>
</html>
HTML
