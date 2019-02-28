import 'package:flutter/material.dart';

class AvatarIcon extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return IconButton(
        onPressed: () {},
        padding: const EdgeInsets.only(top: 13, right: 9, left: 9, bottom: 13),
        icon: Stack(
          children: <Widget>[
            Material(
                shape: CircleBorder(),
                child: CircleAvatar(
                  backgroundImage: AssetImage('assets/test2.png'),
                )),
            Positioned(
                top: 0.0,
                left: 0.0,
                child: Icon(Icons.brightness_1,
                    size: 8.0, color: Colors.redAccent))
          ],
        ));
  }
}
