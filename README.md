# fngr

An implementation of finger built for a more modern world than the original one

this is very sloppily thrown together

## how it expects urls

`/login?username=foo&key=bar`

`/finger?user=username`

---
supports `login`, `logoff`, `bump`, `finger`, `list`, `register`, `check`

planned: `deregister`

## examples

### login

Allows you to login to your account. Your status will be set online for one hour without bumping. Setting `status` is optional, your previous one will carry over.

```
/login?username=foo&key=bar&status=hello
```

### logoff

 Allows you to manually set yourself offline

 ```
 /logoff?username=foo&key=bar
 ```

 ### bump

 Allows you to maintain an online status for over an hour. You must keep bumping at least once an hours to maintain the online status.

 ```
 /bump?username=foo&key=bar
 ```

 ### finger

 Allows you to check the status of a user. Users can see a list of who checks their status using `check` if the other user authenticates.

```
/finger?user=baf&username=foo&key=bar
```

 anonymouse:

 ```
 /finger?user=foo
 ```

 ### list

 Allows you to see a list of users on a server

 ```
 /list
 ```

 ### register

 Allows you to register an account on the server. Server replies with you UUID. This UUID is your authentication key.
 Some servers won't have open registration. Registration can be disabled or require a registration key using `key`.

 #### open

 ```
 /register?username=foo
 ```

 #### registration key

```
/register?username=foo&key=bar
```

### check

Allows you to see what users on the server have checked your status

```
/check?username=foo&key=bar
```
