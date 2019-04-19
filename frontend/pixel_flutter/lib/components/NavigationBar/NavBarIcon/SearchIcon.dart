import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/colors.dart';

class SearchIcon extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return IconButton(
        onPressed: (){},
        color: primaryColor,
        padding: const EdgeInsets.only(right: 9),
        iconSize: 30,
        icon: Icon(Icons.search));
  }
}
