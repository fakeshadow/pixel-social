import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/colors.dart';
import 'package:pixel_flutter/style/text.dart';

class AddPostButton extends StatelessWidget {
  final String text;
  AddPostButton({@required this.text});

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.bottomRight,
      child: Container(
        padding: EdgeInsets.symmetric(horizontal: 50, vertical: 20),
        child: Text('New Topic',style: postButtonStyle,textAlign: TextAlign.center,),
        decoration: BoxDecoration(
            color: primaryColor,
            borderRadius: BorderRadius.only(topLeft: Radius.circular(44))
        ),
      ),
    );
  }
}