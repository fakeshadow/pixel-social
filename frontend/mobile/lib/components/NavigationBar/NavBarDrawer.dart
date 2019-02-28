import 'package:flutter/material.dart';

class NavBarDrawer extends StatelessWidget {
  @override
  Widget build(context) {
    return Drawer(
      semanticLabel: 'Test Drawer',
      child: Flex(
        direction: Axis.vertical,
        children: <Widget>[
          _buildInfoPanel(context),
          Center(),
        ],
      ),
    );
  }
}

Widget _buildInfoPanel(context) {
  return Flex(
    direction: Axis.vertical,
    children: <Widget>[
      Container(
        padding: EdgeInsets.only(top: 30.0),
        width: MediaQuery.of(context).size.width,
        color: Colors.black,
        child: FittedBox(
            fit: BoxFit.scaleDown,
            child: Text(
              'test',
              style: TextStyle(
                fontSize: 30.0,
                fontWeight: FontWeight.bold,
                color: Colors.white,
              ),
            )),
      )
    ],
  );
}
