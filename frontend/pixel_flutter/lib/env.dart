class env {

  /// main service ip
  static const  String url = 'http://192.168.1.197:3200/';

  static const String WS_URL = 'ws://192.168.1.197:3200/talk';

  /// how often user can fetch categories data in seconds
  static const int TIME_GATE = 30;

  /// Max length of topic title
  static const int MAX_TITLE_LENGTH = 256;

  /// Max length of topic body of post content
  static const int MAX_TEXT_LENGTH = 9999;

  /// breakpoint between lg and md size
  static const double BREAK_POINT_WIDTH = 1000.0;

  /// breakpoint between web and mobile
  static const double BREAK_POINT_WIDTH_SM = 600.0;

  /// Snackbar message for success input
  static const String GOT_TOPIC = 'Modified topic success';
}