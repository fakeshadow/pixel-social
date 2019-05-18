import 'package:flutter_web/material.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/style/colors.dart';
import 'package:pixel_flutter_web/style/text.dart';

class InputPage extends StatelessWidget with env {
  final Function onCancelButtonPressed;

  InputPage({@required this.onCancelButtonPressed});

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text('Start a new topic'),
      contentPadding: EdgeInsets.all(16),
      content: Container(
        width: MediaQuery.of(context).size.width < BREAK_POINT_WIDTH_SM
            ? MediaQuery.of(context).size.width
            : BREAK_POINT_WIDTH_SM,
        child: Column(
          mainAxisSize: MainAxisSize.max,
          children: <Widget>[
            TextField(
              autofocus: true,
              decoration: InputDecoration(
                  labelText: 'Title',
                  hintText: 'please input your topic title'),
            ),
            TextField(
              autofocus: false,
              decoration: InputDecoration(
                  labelText: 'Body', hintText: 'please input your topic body'),
            ),
          ],
        ),
      ),
      actions: <Widget>[
        FlatButton(
          onPressed: onCancelButtonPressed,
          child: Text(
            'Cancel',
            style: recoverButtonStyle,
          ),
        ),
        RaisedButton(
          color: primaryColor,
          onPressed: () => Navigator.pop(context, true),
          child: Text(
            'Confirm',
            style: submitButtonStyle.copyWith(fontSize: 16),
          ),
        )
      ],
    );
  }
}
