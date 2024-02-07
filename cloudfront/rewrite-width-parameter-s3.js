function handler(event) {
    /* Modify this array into the available sizes */
    const sizes = [ 500 ];
    
    /*------------*/
    var request = event.request;
    var uri = request.uri;
    var querystring = request.querystring;
    
    // Check if the 'width' query parameter exists
    if (querystring.width && querystring.width.value) {
        var width = querystring.width.value;
        
        var parsedWidth = parseInt(width);
        if (!sizes.includes(parsedWidth)) {
            return request;
        }

        // Modify the URI to include the width in the file name
        var pathParts = uri.split('.');
        if (pathParts.length > 1) {
            var extension = pathParts.pop();
            var newPath = pathParts.join('.') + '_rrs_w' + width + '.' + extension;
            request.uri = newPath;
        }
    }

    return request;
}

