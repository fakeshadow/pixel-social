'use strict'

exports.authPreHook = async (req, res) => {
    try {
        await req.jwtVerify()
    } catch (err) {
        res.send(err)
    }
}
