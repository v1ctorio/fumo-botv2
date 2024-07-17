//Script that downloads a bunch of images url from a Array of urls

const fs = require('fs');
const { get } = require('https');
const { URL } = require('url');

const urls = JSON.parse(fs.readFileSync('attachments.txt'))

urls.forEach(url => {
    console.log("Trying to download", url)
    const filename = getFileName(url);
    const file = fs.createWriteStream(`./downloads/${filename}`);
    
    get(url, response => {
        response.pipe(file);
        console.log("Downloaded", url)
    })
});

function getFileName(url) {
    const parsed = new URL(url);

    const extension = parsed.pathname.split('.').pop();
    const id = parsed.pathname.split('/')[3];
    console.log(extension)
    return id+'.'+extension;
}

