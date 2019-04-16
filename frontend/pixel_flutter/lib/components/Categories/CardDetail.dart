import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/text.dart';

class CardDetail extends StatelessWidget {
  final String categoryTheme;

  CardDetail({this.categoryTheme});

  @override
  Widget build(BuildContext context) {
    return ClipPath(
      clipper: CardDetailClipper(),
      child: Container(
        color: Colors.white,
        height: 180,
        padding: EdgeInsets.only(
            left: 20.0, right: 16.0, top: 24.0, bottom: 12.0),
        child: Column(
          children: <Widget>[
            Align(
              alignment: Alignment.topRight,
              child: Column(
                children: <Widget>[
                  Container(
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(
                        color: Colors.grey.withOpacity(0.4),
                      ),
                    ),
                    height: 40,
                    width: 40,
                    child: Center(
                      child: Text(
                        '31',
                      ),
                    ),
                  ),
                  Text('new')
                ],
              ),
            ),
            SizedBox(
              height: 40.0,
            ),
            Row(
              mainAxisSize: MainAxisSize.max,
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: <Widget>[
                DetailLabel(
                  value: "100",
                  label: 'topics',
                  labelStyle: labelTextStyle,
                  valueStyle: valueTextStyle,
                ),
                DetailLabel(
                  value: "100",
                  label: 'posts',
                  labelStyle: labelTextStyle,
                  valueStyle: valueTextStyle,
                ),
                DetailLabel(
                  value: "100",
                  label: 'subs',
                  labelStyle: labelTextStyle,
                  valueStyle: valueTextStyle,
                )
              ],
            )
          ],
        ),
      ),
    );
  }
}

class DetailLabel extends StatelessWidget {
  final String label, value;
  final TextStyle labelStyle, valueStyle;

  DetailLabel({this.label, this.value, this.labelStyle, this.valueStyle});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text(value, style: valueStyle),
        Text(label, style: labelStyle)
      ],
    );
  }
}


class CardDetailClipper extends CustomClipper<Path> {
  final double distanceY = 12;
  final double controlPoint = 2;

  @override
  Path getClip(Size size) {
    final double _height = size.height;
    final double _width = size.width;

    Path clippedPath = Path();
    clippedPath.moveTo(0, _height * 0.5);
    clippedPath.lineTo(0, _height - distanceY);
    clippedPath.quadraticBezierTo(
      controlPoint,
      _height - controlPoint,
      distanceY,
      _height,
    );
    clippedPath.lineTo(_width, _height);
    clippedPath.lineTo(_width, 30.0);
    clippedPath.quadraticBezierTo(_width - 5, 5, _width - 35, 15);
    clippedPath.close();
    return clippedPath;
  }

  @override
  bool shouldReclip(CustomClipper<Path> oldClipper) {
    return true;
  }
}
