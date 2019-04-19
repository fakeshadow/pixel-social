import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/colors.dart';

class AuthNavBar extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return AppBar(
      backgroundColor: Colors.transparent,
      elevation: 0,
      leading: IconButton(
        onPressed: () => Navigator.pop(context),
        icon: Icon(
          Icons.arrow_back,
          color: primaryColor,
        ),
      ),
    );
  }
}