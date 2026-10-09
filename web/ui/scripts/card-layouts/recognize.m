#import <Foundation/Foundation.h>
#import <Vision/Vision.h>
int main(int argc,const char* argv[]) { @autoreleasepool {
for(int i=1;i<argc;i++) {
 NSString *path=[NSString stringWithUTF8String:argv[i]];
 VNRecognizeTextRequest *request=[VNRecognizeTextRequest new];
 request.recognitionLevel=VNRequestTextRecognitionLevelAccurate;
 request.usesLanguageCorrection=NO;
 VNImageRequestHandler *handler=[[VNImageRequestHandler alloc] initWithURL:[NSURL fileURLWithPath:path] options:@{}];
 NSError *error=nil; [handler performRequests:@[request] error:&error];
 if(error){fprintf(stderr,"%s\n",[[error description] UTF8String]);return 1;}
 NSMutableArray *lines=[NSMutableArray array];
 for(VNRecognizedTextObservation *observation in request.results) {
 VNRecognizedText *text=[[observation topCandidates:1] firstObject]; CGRect b=observation.boundingBox;
 if(text && text.string.length<8) {
 NSMutableArray *characters=[NSMutableArray array];
 for(NSUInteger c=0;c<text.string.length;c++) {
 VNRectangleObservation *glyph=[text boundingBoxForRange:NSMakeRange(c,1) error:nil];
 if(!glyph)continue;CGRect g=glyph.boundingBox;
 [characters addObject:@{@"text":[text.string substringWithRange:NSMakeRange(c,1)],@"x":@(g.origin.x),@"y":@(1-g.origin.y-g.size.height),@"width":@(g.size.width),@"height":@(g.size.height)}];
 }
 [lines addObject:@{@"text":text.string,@"confidence":@(text.confidence),@"x":@(b.origin.x),@"y":@(1-b.origin.y-b.size.height),@"width":@(b.size.width),@"height":@(b.size.height),@"characters":characters}];
 } else if(text)[lines addObject:@{@"text":text.string,@"confidence":@(text.confidence),@"x":@(b.origin.x),@"y":@(1-b.origin.y-b.size.height),@"width":@(b.size.width),@"height":@(b.size.height)}];
 }
 NSData *data=[NSJSONSerialization dataWithJSONObject:@{@"path":path,@"lines":lines} options:NSJSONWritingSortedKeys error:&error];
 puts([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
}
}return 0;}
