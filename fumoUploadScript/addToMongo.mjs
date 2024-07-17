import { MongoClient } from 'mongodb';
import { readdirSync } from 'fs';
import { config } from 'dotenv';

config()
const fumosNames = readdirSync('./downloads')

const client = new MongoClient(process.env.MONGO_URI)
client.on("open", _=>console.log("Connected to the database"))
client.on('commandStarted', start => console.log(start));


const fumosObjects = fumosNames.map(name=>{
    let extension = name.split('.')[1]
    let type = "unknown"
    if (extension == 'mp4') type = 'video'
    if (extension == 'png' || extension == 'jpg') type = 'image'
    if (extension == 'gif') type = 'gif'
    return {
        _id: name.split('.')[0],
        url: `https://cdn.nosesisaid.com/${name}`,
        type
    }
})

await client.connect()
const col = client.db('fumo-api').collection('fumos')
await col.insertMany(fumosObjects)
console.log("Inserted all fumos")