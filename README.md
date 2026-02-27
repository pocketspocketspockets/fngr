# Snuggle

Simple social media inspired by finger

 ## Endpoints

 client to server endpoints
 - `/info`
 - `/login`
 - `/logoff`
 - `/snuggle`
 - `/check`
 - `/bump`
 - `/list`
 - `/register`
 - `/deregister`
 - `/setbio`
 - `/addsocial`
 - `/delsocial`
 - `/setweb`

 server to server endpoints
 - `/fed_snuggle`
 - `/fingerprint`

 ## Response

 Server can reply with
 - 200 on success
 - 404 on user isn't found
 - 400 on improperly formated request
 - 401 on unauthorized
 - 500 on server failure

 All error responses are a JSON object

 ```
 {
     "Error": String
 }
 ```
