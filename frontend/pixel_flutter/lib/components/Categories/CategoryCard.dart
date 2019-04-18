import 'package:flutter/material.dart';
import 'package:pixel_flutter/Views/TopicsPage.dart';
import 'package:pixel_flutter/components/Categories/CardName.dart';
import 'package:pixel_flutter/components/Categories/CardDetail.dart';
import 'package:pixel_flutter/models/Category.dart';

/// push to topics page if from this widget.
class CategoryCard extends StatelessWidget {
  final Category category;

  CategoryCard({this.category});

  Widget build(BuildContext context) {
    return Hero(
      tag: category.name,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: () => Navigator.push(
              context,
              MaterialPageRoute(
                  builder: (context) => TopicsPage(
                        category: category,
                      ))),
          child: SizedBox(
              width: 280,
              child: Card(
                margin: EdgeInsets.symmetric(horizontal: 25.0, vertical: 60.0),
                elevation: 20.0,
                shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.all(Radius.circular(20.0))),
                child: ClipRRect(
                  borderRadius: BorderRadius.all(Radius.circular(20.0)),
                  child: Stack(
                    children: <Widget>[
                      // ToDo: Future look into fade image loading
                      FadeInImage.assetNetwork(
                        placeholder: 'assets/test2.png',
                        image: category.theme,
                        fit: BoxFit.fitWidth,
                      ),
                      Positioned(
                        left: 0,
                        bottom: 0,
                        right: 0,
                        child: CardDetail(),
                      ),
                      Positioned(
                          left: 0,
                          bottom: 70,
                          child: InkWell(
                            onTap: () => {},
                            child: CardName(
                              cardName: category.name,
                            ),
                          )),
                    ],
                  ),
                ),
              )),
        ),
      ),
    );
  }
}
