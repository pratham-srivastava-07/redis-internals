for redis to understand what a client wants, it uses RESP protocol, it supports data types such as int, string, arrays and ways of conveying errors.

Resp is a req-res protocol that when client sends request, receives res, is a RESP. (every data type starts with special char and ends with \r\n)

RESP encoding for diff datat types:

for strings: starts with '+' e.g. +PING\r\n

integers: starts with ':' e.g. :15\r\n

Bulk strings: starts with '$' e.g. $4\r\nPONG\r\n

Arrays: starts with '*' e.g. if we have ["i", 500, "newone"]
                          the encoding:  *3\r\n
                                         $1\r\ni\r\n
                                         :200\r\n
                                         $6\r\newone\r\n
    
Empty starts with whatever datatype we have first and then 0, CRLF. e.g. empty string -> +0\r\n
                                                                         empty  array -> *0\r\n
                                                                         null values -> *-1\r\n

Simple strings are not binary safe, they can contain any byte, cannot contain \r\n, can be used to store any binary data, even an image


Anything that  starts with a - is an error being represented in RESP. e.g: -{msg}\r\n
