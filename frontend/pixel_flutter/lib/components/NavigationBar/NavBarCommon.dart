import 'package:flutter/material.dart';
import './NavBarIcon/SearchIcon.dart';
import './NavBarIcon/AvatarIcon.dart';

class NavBarCommon extends StatelessWidget {
  
  final String title;
  final bool isClose;

  NavBarCommon({
    this.title,
    this.isClose,
  });

  @override
  Widget build(BuildContext context) {
    return SliverAppBar(
        leading: isClose == true
            ? IconButton(
                icon: Icon(Icons.close),
                tooltip: 'Go back',
                onPressed: Navigator.of(context).pop,
              )
            : Container(),
        floating: true,
        snap: true,
        title: Text(title != null ? title : 'PixelShare'),
        centerTitle: true,
        actions: <Widget>[SearchIcon(), AvatarIcon()]);
  }
}
