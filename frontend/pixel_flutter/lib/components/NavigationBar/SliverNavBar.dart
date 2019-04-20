import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/colors.dart';
import 'package:pixel_flutter/components/Icon/SearchIcon.dart';
import 'package:pixel_flutter/components/Icon/AvatarIcon.dart';

// ToDo: SliverNavBar rebuild multiple times with unknown reason
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
          color: primaryColor,
          icon: Icon(Icons.arrow_back),
          tooltip: 'Go back',
          onPressed: Navigator.of(context).pop,
        ),
        floating: true,
        elevation: 0,
        snap: true,
        backgroundColor: Colors.transparent,
        title: FadeInImage.assetNetwork(
            placeholder: 'assets/test2.png',
            image: theme,
            fit: BoxFit.cover,
          ),
        centerTitle: true,
        actions: <Widget>[SearchIcon(), AvatarIcon()]);
  }
}
