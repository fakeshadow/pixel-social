'use strict'

exports.authPreHandler = async (req, res) => {
    try {
        await req.jwtVerify()
    } catch (err) {
        res.send(err)
    }
}
