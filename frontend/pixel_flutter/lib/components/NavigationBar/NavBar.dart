import 'package:flutter/material.dart';
import 'package:pixel_flutter/components/Icon/AvatarIcon.dart';

class NavBar extends StatelessWidget {
  final String title;

  NavBar({
    this.title,
  });

  @override
  Widget build(BuildContext context) {
    return AppBar(
      elevation: 0,
        toolbarOpacity: 1,
        bottomOpacity: 0,
        backgroundColor: Colors.transparent,
        actions: <Widget>[AvatarIcon()]);
  }
}