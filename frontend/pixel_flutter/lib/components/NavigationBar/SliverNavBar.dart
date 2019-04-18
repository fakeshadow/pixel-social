import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/colors.dart';
import './NavBarIcon/SearchIcon.dart';
import 'package:pixel_flutter/components/Icon/AvatarIcon.dart';

class SliverNavBar extends StatelessWidget {
  final String title;
  final String theme;

  SliverNavBar({
    this.title,
    this.theme
  });

  @override
  Widget build(BuildContext context) {
    return SliverAppBar(
        leading: IconButton(
          icon: Icon(Icons.arrow_back),
          tooltip: 'Go back',
          onPressed: Navigator.of(context).pop,
        ),
        floating: true,
        snap: true,
        backgroundColor: primaryColor,
        title: FadeInImage.assetNetwork(
            placeholder: 'assets/test2.png',
            image: theme,
            fit: BoxFit.cover,
          ),
        centerTitle: true,
        actions: <Widget>[SearchIcon(), AvatarIcon()]);
  }
}
