import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/colors.dart';
import 'package:pixel_flutter/style/text.dart';

class CardName extends StatelessWidget {
  final String cardName;

  CardName({this.cardName});

  @override
  Widget build(BuildContext context) {
    return Material(
      color: primaryColor,
      elevation: 20.0,
      shape: CustomShapeBorder(),
      child: Padding(
        padding: EdgeInsets.all(21),
        child: Text(
          cardName,
          textAlign: TextAlign.center,
          style: categoryCardNameStyle,
        ),
      ),
    );
  }
}

class CustomShapeBorder extends ShapeBorder {
  final double distanceY = 12;
  final double distanceControlPoint = 2;

  @override
  EdgeInsetsGeometry get dimensions => null;

  @override
  Path getInnerPath(Rect rect, {TextDirection textDirection}) {
    return null;
  }

  @override
  Path getOuterPath(Rect rect, {TextDirection textDirection}) {
    return getClip(Size(130.0, 60.0));
  }

  @override
  void paint(Canvas canvas, Rect rect, {TextDirection textDirection}) {}

  @override
  ShapeBorder scale(double t) {
    return null;
  }

  Path getClip(Size size) {
    final double _height = size.height;
    final double _width = size.width;

    Path clippedPath = Path();
    clippedPath.moveTo(distanceY, 0);
    clippedPath.quadraticBezierTo(
        distanceControlPoint, distanceControlPoint, 0, distanceY);
    clippedPath.lineTo(0, _height - distanceY);
    clippedPath.quadraticBezierTo(distanceControlPoint,
        _height - distanceControlPoint, distanceY, _height);
    clippedPath.lineTo(_width - distanceY, _height);
    clippedPath.quadraticBezierTo(_width - distanceControlPoint,
        _height - distanceControlPoint, _width, _height - distanceY);
    clippedPath.lineTo(_width, _height * 0.6);
    clippedPath.quadraticBezierTo(_width - 1, _height * 0.6 - distanceY,
        _width - distanceY, _height * 0.6 - distanceY - 3);
    clippedPath.lineTo(distanceY + 6, 0);
    clippedPath.close();
    return clippedPath;
  }
}
