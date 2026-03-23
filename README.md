moved to codeberg: https://codeberg.org/pockets/snuggle


# Snuggle

Simple federated social media inspired by finger. Snuggling lets you see a user's profile, and they get to see yours.

## Info

there is a lot here changing for 0.4a, including stable 1.0 API version. This README is not up to date after the feature list.

## Features

- [x] info
- [x] login
- [x] logoff
- [x] snuggle
- [x] check
- [x] bump
- [x] list
- [x] register
- [x] deregister
- [x] setbio
- [x] addsocial
- [x] delsocial
- [x] setweb
- [x] fed
- [x] fed/fingerprint
- [x] profiles
    - [x] website
    - [x] social media list
    - [x] bio
- [x] status
    - [x] state
    - [x] message
    - [x] bump
- [x] offline worker

!!! all need documentation for 1.0 !!!

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
 - `/fed`
 - `/fed/fingerprint`

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
