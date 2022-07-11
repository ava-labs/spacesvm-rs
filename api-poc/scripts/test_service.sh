echo "Example methods"

printf "Build block:\n"
curl -X POST \
     -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","id":"id","method":"build_block"}' \
     http://localhost:9001

printf "\nGet block:\n"
curl -X POST \
     -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","id":"id","method":"get_block", 
            "params":[{
                "id":{
                    "placehold":"39"
                }
            }]
        }' \
     http://localhost:9001

printf "\nSet preference:\n"
curl -X POST \
     -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","id":"id","method":"set_preference", 
            "params":[{
                "id":{
                    "placehold":"4096"
                }
            }]
        }' \
     http://localhost:9001

printf "\nSet state:\n"
curl -X POST \
     -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","id":"id","method":"set_state", 
            "params":[{
                "state":2
            }]
        }' \
     http://localhost:9001

printf "\nLast accepted:\n"
curl -X POST \
     -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","id":"id","method":"last_accepted"}' \
     http://localhost:9001

printf "\nSet state:\n"
curl -X POST \
     -H 'Content-Type: application/json' \
     -d '{"jsonrpc":"2.0","id":"id","method":"parse_block", 
            "params":[{
                "bytes": [75, 111, 104, 110]
            }]
        }' \
     http://localhost:9001