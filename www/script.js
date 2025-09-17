// Localhost HTTP Server - Client-side JavaScript
console.log('🚀 Localhost HTTP Server - Static file serving working!');

// Add some interactive functionality
document.addEventListener('DOMContentLoaded', function() {
    // Add click handlers to test links
    const testLinks = document.querySelectorAll('.test-links a');
    
    testLinks.forEach(link => {
        link.addEventListener('click', function(e) {
            console.log(`Testing: ${this.href}`);
        });
    });
    
    // Add a simple animation to the header
    const header = document.querySelector('header h1');
    if (header) {
        header.style.transition = 'transform 0.3s ease';
        header.addEventListener('mouseenter', function() {
            this.style.transform = 'scale(1.05)';
        });
        header.addEventListener('mouseleave', function() {
            this.style.transform = 'scale(1)';
        });
    }
    
    // Display server info
    const serverInfo = {
        timestamp: new Date().toISOString(),
        userAgent: navigator.userAgent,
        language: navigator.language,
        platform: navigator.platform
    };
    
    console.log('Client Info:', serverInfo);
    
    // Test fetch API with our server
    fetch('/test.json')
        .then(response => {
            console.log('Fetch test response:', response.status, response.statusText);
            return response.json().catch(() => null);
        })
        .then(data => {
            if (data) {
                console.log('JSON data received:', data);
            }
        })
        .catch(error => {
            console.log('Fetch test (expected if test.json doesn\'t exist):', error.message);
        });
});
